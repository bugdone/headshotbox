use std::collections::{HashMap, HashSet};

use csgo_demo::proto::netmessages::CSVCMsg_GameEventList;

pub(super) struct DescriptorKey {
    pub type_: i32,
    pub name: String,
}

pub(super) struct Descriptor {
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

pub(super) fn parse_game_event_list(gel: CSVCMsg_GameEventList) -> HashMap<i32, Descriptor> {
    let hsbox_events = HashSet::from([
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
                Descriptor {
                    name: d.name().to_string(),
                    keys: d
                        .keys
                        .iter()
                        .map(|k| DescriptorKey {
                            name: k.name().to_string(),
                            type_: k.type_(),
                        })
                        .collect(),
                },
            )
        })
        .collect()
}
