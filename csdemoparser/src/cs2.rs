use crate::demoinfo::{
    BombDefused, BombExploded, Event, EventTick, PlayerDeath, PlayerDisconnect, PlayerHurt,
    RoundEnd, RoundStart,
};

use crate::game_event::GameEvent;
use crate::last_jump::LastJump;
use crate::{DemoInfo, Slot, UserId};
use cs2_demo::entity::Entities;
use cs2_demo::proto::demo::CDemoFileHeader;
use cs2_demo::proto::gameevents::CMsgSource1LegacyGameEvent;
use cs2_demo::{GameEventDescriptors, UserInfo, Visitor};
use demo_format::Tick;
use std::collections::{hash_map, HashMap};
use tracing::{instrument, trace};

pub fn parse(read: &mut dyn std::io::Read) -> anyhow::Result<DemoInfo> {
    let mut state = GameState::new();
    cs2_demo::parse_after_demo_type(read, &mut state)?;
    state.get_info()
}

#[derive(Default)]
struct GameState {
    game_event_descriptors: GameEventDescriptors,
    last_jump: LastJump<UserId>,
    /// Maps player user_id to slot.
    user_id2slot: HashMap<UserId, Slot>,
    /// Maps player slot to player info.
    players: HashMap<Slot, cs2_demo::PlayerInfo>,

    demoinfo: DemoInfo,
    // DemoInfo field
    events: Vec<EventTick>,
}

impl Visitor for GameState {
    fn visit_file_header(&mut self, header: CDemoFileHeader) -> anyhow::Result<()> {
        self.demoinfo.servername = header.server_name().to_string();
        self.demoinfo.map = header.map_name().to_string();
        Ok(())
    }

    fn visit_server_info(
        &mut self,
        server_info: cs2_demo::proto::netmessages::CSVCMsg_ServerInfo,
    ) -> anyhow::Result<()> {
        self.demoinfo.tickrate = server_info.tick_interval();
        Ok(())
    }

    fn visit_userinfo_table(&mut self, st: Vec<UserInfo>) -> anyhow::Result<()> {
        for ui in st {
            self.update_players(ui);
        }
        Ok(())
    }

    fn visit_game_event(
        &mut self,
        event: CMsgSource1LegacyGameEvent,
        tick: Tick,
        _entities: &Entities,
    ) -> anyhow::Result<()> {
        if let Some(descriptor) = self.game_event_descriptors.get(&event.eventid()) {
            let event = cs2_demo::game_event::de::from_proto(event, descriptor)?;
            self.handle_game_event(event, tick)?;
        }
        Ok(())
    }

    fn visit_game_event_descriptors(
        &mut self,
        mut descriptors: GameEventDescriptors,
    ) -> anyhow::Result<()> {
        let hsbox_events = std::collections::HashSet::from([
            "bomb_defused",
            "bomb_exploded",
            "bot_takeover",
            "game_restart",
            "player_connect",
            "player_death",
            "player_disconnect",
            "player_hurt",
            "player_jump",
            "player_spawn",
            "round_end",
            "round_officially_ended",
            "round_start",
            "score_changed",
            "smokegrenade_detonate",
            "smokegrenade_expired",
        ]);
        descriptors.retain(|_, ed| hsbox_events.contains(ed.name.as_str()));
        self.game_event_descriptors = descriptors;
        Ok(())
    }
}

impl GameState {
    fn new() -> Self {
        Default::default()
    }

    fn get_info(mut self) -> anyhow::Result<DemoInfo> {
        self.demoinfo.events = self
            .events
            .iter()
            .map(serde_json::to_value)
            .collect::<std::result::Result<_, _>>()?;
        Ok(self.demoinfo)
    }

    fn add_event(&mut self, tick: Tick, event: Event) {
        self.events.push(EventTick { tick, event })
    }

    /// Returns the user XUID if available.
    ///
    /// userid 65535 is used as a marker for events where there is no alive player, for example:
    /// - kills with no assister
    /// - player disconnected
    /// - player died, for example before the smoke_expired event
    fn maybe_xuid(&self, userid: i32) -> u64 {
        let Some(slot) = self.user_id2slot.get(&UserId(userid as u16)) else {
            return userid as u64;
        };
        if let Some(player) = self.players.get(slot) {
            return player.xuid;
        }
        userid as u64
    }

    #[instrument(level = "trace", skip_all)]
    fn handle_game_event(&mut self, ge: GameEvent, tick: Tick) -> anyhow::Result<()> {
        trace!("#{tick} GameEvent {:?}", ge);
        match ge {
            GameEvent::BombDefused(e) => {
                let userid = self.maybe_xuid(e.userid);
                self.add_event(tick, Event::BombDefused(BombDefused { userid }))
            }
            GameEvent::BombExploded(e) => {
                let userid = self.maybe_xuid(e.userid);
                self.add_event(tick, Event::BombExploded(BombExploded { userid }))
            }
            GameEvent::PlayerDeath(e) => {
                // TODO: add processing
                let userid = self.maybe_xuid(e.userid);
                let attacker = self.maybe_xuid(e.attacker);
                let assister = self.maybe_xuid(e.assister);
                let jump = self.last_jump.ticks_since_last_jump(
                    UserId(e.attacker as u16),
                    tick,
                    self.demoinfo.tickrate,
                );
                self.add_event(
                    tick,
                    Event::PlayerDeath(PlayerDeath {
                        userid,
                        attacker,
                        assister,
                        assistedflash: e.assistedflash,
                        weapon: e.weapon,
                        headshot: e.headshot,
                        penetrated: e.penetrated,
                        noscope: e.noscope,
                        thrusmoke: e.thrusmoke,
                        attackerblind: e.attackerblind,
                        distance: e.distance,
                        jump,
                    }),
                )
            }
            GameEvent::PlayerConnect(_) => {
                // TODO: processing
            }
            GameEvent::PlayerDisconnect(e) => {
                // TODO: processing
                let userid = self.maybe_xuid(e.userid);
                self.add_event(tick, Event::PlayerDisconnected(PlayerDisconnect { userid }))
            }
            GameEvent::PlayerHurt(e) => {
                let userid = self.maybe_xuid(e.userid);
                let attacker = self.maybe_xuid(e.attacker);
                self.add_event(
                    tick,
                    Event::PlayerHurt(PlayerHurt {
                        userid,
                        attacker,
                        dmg_health: e.dmg_health,
                    }),
                )
            }
            GameEvent::PlayerJump(e) => {
                self.last_jump.record_jump(UserId(e.userid as u16), tick);
            }
            GameEvent::PlayerSpawn(_) => {
                // In CS:GO, player_spawn was used to determine the team composition
                // for each round. But in CS2, the PlayerSpawn event doesn't have a
                // teamnum, so it is useless.
            }
            GameEvent::RoundStart(e) => {
                // TODO: add processing
                self.add_event(
                    tick,
                    Event::RoundStart(RoundStart {
                        timelimit: e.timelimit,
                    }),
                )
            }
            GameEvent::RoundEnd(e) => self.add_event(
                tick,
                Event::RoundEnd(RoundEnd {
                    winner: e.winner,
                    reason: e.reason,
                    message: e.message,
                }),
            ),
            GameEvent::RoundOfficiallyEnded => self.add_event(tick, Event::RoundOfficiallyEnded),
            // In CS2 we always have player_death.thrusmoke.
            GameEvent::SmokegrenadeDetonate(_) => (),
            GameEvent::SmokegrenadeExpired(_) => (),
        }
        Ok(())
    }

    fn update_players(&mut self, ui: UserInfo) {
        let slot = Slot(ui.index);
        if let hash_map::Entry::Vacant(e) = self.players.entry(slot) {
            self.demoinfo
                .player_names
                .insert(ui.info.xuid.to_string(), ui.info.name.clone());
            self.demoinfo
                .player_slots
                .insert(ui.info.xuid.to_string(), ui.info.user_id);
            self.user_id2slot
                .insert(UserId(ui.info.user_id as u16), slot);
            e.insert(ui.info);
        }
    }
}
