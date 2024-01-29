pub mod de;

use std::collections::HashMap;

use crate::proto::gameevents::{
    cmsg_source1legacy_game_event_list, CMsgSource1LegacyGameEventList,
};

pub type GameEventDescriptors = HashMap<i32, Descriptor>;

pub struct DescriptorKey {
    pub type_: i32,
    pub name: String,
}

impl From<cmsg_source1legacy_game_event_list::Key_t> for DescriptorKey {
    fn from(k: cmsg_source1legacy_game_event_list::Key_t) -> Self {
        Self {
            name: k.name().to_string(),
            type_: k.type_(),
        }
    }
}

pub struct Descriptor {
    pub eventid: i32,
    pub name: String,
    pub keys: Vec<DescriptorKey>,
}

impl From<cmsg_source1legacy_game_event_list::Descriptor_t> for Descriptor {
    fn from(d: cmsg_source1legacy_game_event_list::Descriptor_t) -> Self {
        Descriptor {
            eventid: d.eventid(),
            name: d.name().to_string(),
            keys: d.keys.into_iter().map(DescriptorKey::from).collect(),
        }
    }
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

pub(crate) fn parse_game_event_list(gel: CMsgSource1LegacyGameEventList) -> GameEventDescriptors {
    gel.descriptors
        .into_iter()
        .map(|d| (d.eventid(), Descriptor::from(d)))
        .collect()
}
