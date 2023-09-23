use crate::demoinfo::{
    BombDefused, BombExploded, Event, EventTick, PlayerDeath, PlayerDisconnect, PlayerHurt,
    RoundEnd, RoundStart,
};

use crate::game_event::{from_cs2_event, parse_game_event_list_impl, GameEvent};
use crate::{game_event, DemoInfo};
use cs2_demo::packet::Message;
use cs2_demo::proto::demo::CDemoFileHeader;
use cs2_demo::proto::gameevents::CMsgSource1LegacyGameEventList;
use cs2_demo::DemoCommand;
use demo_format::Tick;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::rc::Rc;
use tracing::trace;

pub fn parse(read: &mut dyn io::Read) -> anyhow::Result<DemoInfo> {
    let mut parser = cs2_demo::DemoParser::try_new_after_demo_type(read)?;
    let mut state = GameState::new();
    while let Some((tick, cmd)) = parser.parse_next_demo_command()? {
        trace!("t#{tick:?} {cmd}");
        match cmd {
            DemoCommand::FileHeader(header) => {
                state.handle_file_header(header)?;
            }
            DemoCommand::Packet(p) => {
                for msg in p.messages {
                    state.handle_packet(msg, tick)?;
                }
            }
            _ => {}
        }
    }
    state.get_info()
}

#[derive(Default)]
struct GameState {
    game_event_descriptors: HashMap<i32, game_event::Descriptor>,
    events: Vec<EventTick>,
    jumped_last: HashMap<i32, Tick>,
    demoinfo: Rc<RefCell<DemoInfo>>,
}

impl GameState {
    fn new() -> Self {
        GameState {
            ..Default::default()
        }
    }

    fn handle_file_header(&mut self, header: CDemoFileHeader) -> anyhow::Result<()> {
        let mut demoinfo = self.demoinfo.borrow_mut();
        demoinfo.servername = header.server_name().to_string();
        demoinfo.map = header.map_name().to_string();
        Ok(())
    }

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
        Ok(demoinfo.clone())
    }

    fn add_event(&mut self, tick: Tick, event: Event) {
        self.events.push(EventTick { tick, event })
    }

    fn maybe_xuid(&mut self, userid: i32) -> u64 {
        // TODO: lookup self.players
        userid as u64
    }

    fn handle_game_event(&mut self, ge: GameEvent, tick: Tick) -> anyhow::Result<()> {
        trace!("GameEvent {:?}", ge);
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
                self.jumped_last.insert(e.userid, tick);
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
}

parse_game_event_list_impl!(CMsgSource1LegacyGameEventList);
