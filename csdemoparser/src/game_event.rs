#![allow(dead_code)]

mod de;

pub(crate) use de::from_cs2_event;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum GameEvent {
    BombDefused(BombDefused),
    BombExploded(BombExploded),
    PlayerDeath(PlayerDeath),
    PlayerHurt(PlayerHurt),
    PlayerJump(PlayerJump),
    PlayerSpawn(PlayerSpawn),
    PlayerConnect(PlayerConnect),
    PlayerDisconnect(PlayerDisconnect),
    RoundStart(RoundStart),
    RoundEnd(RoundEnd),
    RoundOfficiallyEnded,
    SmokegrenadeDetonate(SmokegrenadeDetonate),
    SmokegrenadeExpired(SmokegrenadeExpired),
}

#[derive(Debug, Deserialize)]
pub(crate) struct BombDefused {
    pub userid: i32, // short, playercontroller
}

#[derive(Debug, Deserialize)]
pub(crate) struct BombExploded {
    pub userid: i32, // short, playercontroller
}

#[derive(Debug, Deserialize)]
pub(crate) struct PlayerConnect {
    pub name: String,
    pub userid: i32, // short, playercontroller
    pub networkid: String,
    pub xuid: u64,
    pub bot: bool,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub(crate) struct PlayerDisconnect {
    pub userid: i32, // short, playercontroller
    pub reason: i32,
    pub name: String,
    pub networkid: String,
    pub xuid: u64,
    pub PlayerID: i32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PlayerHurt {
    pub userid: i32,   // short, playercontroller
    pub attacker: i32, // short, playercontroller
    pub dmg_health: i32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PlayerDeath {
    pub userid: i32,   // short, playercontroller
    pub attacker: i32, // short, playercontroller
    pub assister: i32, // short, playercontroller
    pub assistedflash: bool,
    pub weapon: String,
    pub headshot: bool,
    pub penetrated: i32,
    pub noscope: bool,
    pub thrusmoke: bool,
    pub attackerblind: bool,
    pub distance: f32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PlayerSpawn {
    pub userid: i32, // short, playercontroller
}

#[derive(Debug, Deserialize)]
pub(crate) struct PlayerJump {
    pub userid: i32, // short, playercontroller
}

#[derive(Debug, Deserialize)]
pub(crate) struct RoundStart {
    pub timelimit: i32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RoundEnd {
    pub winner: i32,
    pub reason: i32,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SmokegrenadeDetonate {
    pub userid: i32, // short, playercontroller
    pub entityid: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SmokegrenadeExpired {
    pub userid: i32, // short, playercontroller
    pub entityid: i32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub(crate) struct DescriptorKey {
    pub type_: i32,
    pub name: String,
}

pub(crate) struct Descriptor {
    pub name: String,
    pub keys: Vec<DescriptorKey>,
}

impl std::fmt::Display for Descriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "struct {} {{", self.name)?;
        for key in &self.keys {
            match key.type_ {
                1 => writeln!(f, "    {}: String,", key.name)?,
                2 => writeln!(f, "    {}: f32,", key.name)?,
                3 => writeln!(f, "    {}: i32, // long", key.name)?,
                4 => writeln!(f, "    {}: i32, // short", key.name)?,
                5 => writeln!(f, "    {}: i32, // byte", key.name)?,
                6 => writeln!(f, "    {}: bool,", key.name)?,
                7 => writeln!(f, "    {}: u64,", key.name)?,
                8 => writeln!(f, "    {}: i32, // long, strict_ehandle", key.name)?,
                9 => writeln!(f, "    {}: i32, // short, playercontroller", key.name)?,
                t => writeln!(f, "    {}: <unknown_{t}>", key.name)?,
            };
        }
        writeln!(f, "}}")
    }
}

#[allow(dead_code)]
pub(crate) fn dump_descriptors(descriptors: HashMap<i32, Descriptor>) {
    let mut sorted: Vec<_> = descriptors.values().collect();
    sorted.sort_by_key(|d| &d.name);
    for d in sorted {
        println!("{d}");
    }
}

macro_rules! parse_game_event_list_impl {
    ($game_event_list:ty) => {
        fn parse_game_event_list(
            gel: $game_event_list,
        ) -> HashMap<i32, crate::game_event::Descriptor> {
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
            gel.descriptors
                .into_iter()
                .filter(|d| hsbox_events.contains(d.name()))
                .map(|d| {
                    (
                        d.eventid(),
                        crate::game_event::Descriptor {
                            name: d.name().to_string(),
                            keys: d
                                .keys
                                .iter()
                                .map(|k| game_event::DescriptorKey {
                                    name: k.name().to_string(),
                                    type_: k.type_(),
                                })
                                .collect(),
                        },
                    )
                })
                .collect()
        }
    };
}
pub(crate) use parse_game_event_list_impl;
