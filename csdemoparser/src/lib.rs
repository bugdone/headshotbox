mod cs2;
mod csgo;
pub mod demoinfo;
mod game_event;
mod geometry;
mod last_jump;

use csgo_demo::entity::{Entity, EntityId, PropValue, Scalar};
use demoinfo::DemoInfo;
use std::{
    fs::File,
    io::{Read, Seek},
};

type Tick = i32;

const SOURCE1_DEMO_TYPE: &[u8; 8] = b"HL2DEMO\0";
const SOURCE2_DEMO_TYPE: &[u8; 8] = b"PBDEMS2\0";

pub fn parse(read: &mut File) -> anyhow::Result<DemoInfo> {
    let mut demo_type = [0; 8];
    read.read_exact(&mut demo_type)?;
    read.rewind()?;
    match &demo_type {
        SOURCE1_DEMO_TYPE => csgo::parse(read),
        SOURCE2_DEMO_TYPE => {
            if std::env::var("CS2_EXPERIMENTAL_PARSER").is_ok() {
                cs2::parse(read)
            } else {
                panic!("CS2 demo parser is not complete. You can test it by seting the CS2_EXPERIMENTAL_PARSER environment variable.")
            }
        }
        _ => Err(cs2_demo::Error::InvalidDemoType(Box::new(demo_type)).into()),
    }
}

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
struct Slot(u16);
#[derive(Eq, PartialEq, Hash, Clone, Copy, Default)]
struct UserId(u16);

#[derive(Default)]
struct TeamScore {
    team_entity_id: [Option<EntityId>; 2],
    // score at the start of the round
    round_start: [i32; 2],
    // current score
    score: [i32; 2],
}

impl TeamScore {
    fn update(&mut self, entity: &Entity, value: &PropValue) -> bool {
        let Some(pos) = self
            .team_entity_id
            .iter()
            .position(|i| &Some(entity.id) == i)
        else {
            return false;
        };
        let &PropValue::Scalar(Scalar::I32(new_score)) = value else {
            return false;
        };
        if new_score < self.round_start[0] && new_score < self.round_start[1] {
            return false;
        }
        if self.score[pos] == new_score {
            return false;
        }
        self.score[pos] = new_score;
        true
    }

    fn set_round_start(&mut self) {
        self.round_start = self.score;
    }
}

fn guid_to_xuid(guid: &str) -> anyhow::Result<u64> {
    let high_bits = guid.chars().skip(10).collect::<String>().parse::<i32>()?;
    let low_bit: i32 = if let Some('1') = guid.chars().nth(8) {
        1
    } else {
        0
    };
    let account_id = 2 * high_bits + low_bit;
    Ok(account_id_to_xuid(account_id))
}

fn account_id_to_xuid(account_id: i32) -> u64 {
    account_id as u64 + 76561197960265728
}

fn maybe_get_u16(v: Option<&serde_json::Value>) -> Option<u16> {
    Some(v?.as_i64()? as u16)
}

fn maybe_get_i32(v: Option<&serde_json::Value>) -> Option<i32> {
    Some(v?.as_i64()? as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_guid_to_xuid() {
        assert_eq!(
            guid_to_xuid("STEAM_1:0:30828430").unwrap(),
            76561198021922588
        );
        assert!(guid_to_xuid("BOT").is_err());
    }
}
