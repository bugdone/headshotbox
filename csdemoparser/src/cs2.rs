use crate::demoinfo::{
    BombDefused, BombExploded, Event, EventTick, PlayerDeath, PlayerDisconnect, PlayerHurt,
    RoundEnd, RoundStart,
};

use crate::game_event::{from_cs2_event, parse_game_event_list_impl, GameEvent};
use crate::last_jump::LastJump;
use crate::{game_event, DemoInfo, Slot, UserId};
use cs2_demo::proto::demo::CDemoFileHeader;
use cs2_demo::proto::gameevents::CMsgSource1LegacyGameEventList;
use cs2_demo::{DemoCommand, StringTable};
use cs2_demo::{Message, UserInfo};
use demo_format::Tick;
use std::cell::RefCell;
use std::collections::{hash_map, HashMap};
use std::io;
use std::rc::Rc;
use tracing::{instrument, trace, trace_span};

pub fn parse(read: &mut dyn io::Read) -> anyhow::Result<DemoInfo> {
    let mut parser = cs2_demo::DemoParser::try_new_after_demo_type(read)?;
    let mut state = GameState::new();
    while let Some((tick, cmd)) = parser.parse_next_demo_command()? {
        trace_span!("demo_command").in_scope(|| trace!("#{tick:?} {cmd}"));
        match cmd {
            DemoCommand::FileHeader(header) => {
                state.handle_file_header(header)?;
            }
            DemoCommand::Packet(p) => {
                for msg in p.messages {
                    state.handle_packet(msg, tick)?;
                }
            }
            DemoCommand::StringTables(st) => state.handle_string_tables(st)?,
            // DemoCommand::SendTables(send) => state.handle_send_tables(send)?,
            _ => {}
        }
    }
    state.get_info()
}

#[derive(Default)]
struct GameState {
    game_event_descriptors: HashMap<i32, game_event::Descriptor>,
    last_jump: LastJump<UserId>,
    /// Maps player user_id to slot.
    user_id2slot: HashMap<UserId, Slot>,
    /// Maps player slot to player info.
    players: HashMap<Slot, cs2_demo::PlayerInfo>,

    // DemoInfo fields
    events: Vec<EventTick>,
    player_names: HashMap<String, String>,
    player_slots: HashMap<String, i32>,
    tick_interval: f32,
    demoinfo: Rc<RefCell<DemoInfo>>,
}

impl GameState {
    fn new() -> Self {
        Default::default()
    }

    fn handle_file_header(&mut self, header: CDemoFileHeader) -> anyhow::Result<()> {
        let mut demoinfo = self.demoinfo.borrow_mut();
        demoinfo.servername = header.server_name().to_string();
        demoinfo.map = header.map_name().to_string();
        Ok(())
    }

    #[instrument(level = "trace", skip_all)]
    fn handle_packet(&mut self, msg: Message, tick: Tick) -> anyhow::Result<()> {
        match msg {
            Message::Source1LegacyGameEvent(event) => {
                if let Some(descriptor) = self.game_event_descriptors.get(&event.eventid()) {
                    let event = from_cs2_event(event, descriptor)?;
                    self.handle_game_event(event, tick)?;
                }
            }
            Message::Source1LegacyGameEventList(gel) => {
                self.game_event_descriptors = parse_game_event_list(gel);
            }
            Message::ServerInfo(si) => self.tick_interval = si.tick_interval(),
            Message::Unknown(_) => (),
        }
        Ok(())
    }

    fn get_info(self) -> anyhow::Result<DemoInfo> {
        let mut demoinfo = self.demoinfo.borrow_mut();
        demoinfo.events = self
            .events
            .iter()
            .map(serde_json::to_value)
            .collect::<std::result::Result<_, _>>()?;
        demoinfo.tickrate = self.tick_interval;
        demoinfo.player_names = self.player_names;
        demoinfo.player_slots = self.player_slots;
        Ok(demoinfo.clone())
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
                    self.tick_interval,
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

    fn handle_string_tables(&mut self, st: Vec<StringTable>) -> anyhow::Result<()> {
        for table in st {
            match table {
                StringTable::UserInfo(table) => {
                    for ui in table {
                        self.update_players(ui);
                    }
                }
            }
        }
        Ok(())
    }

    fn update_players(&mut self, ui: UserInfo) {
        let slot = Slot(ui.index);
        if let hash_map::Entry::Vacant(e) = self.players.entry(slot) {
            self.player_names
                .insert(ui.info.xuid.to_string(), ui.info.name.clone());
            self.player_slots
                .insert(ui.info.xuid.to_string(), ui.info.user_id);
            self.user_id2slot
                .insert(UserId(ui.info.user_id as u16), slot);
            e.insert(ui.info);
        }
    }
}

parse_game_event_list_impl!(CMsgSource1LegacyGameEventList);
