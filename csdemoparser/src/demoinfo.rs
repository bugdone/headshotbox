use crate::Tick;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The output of csdemoparser.
#[derive(Serialize, Deserialize, Clone)]
pub struct DemoInfo {
    // TODO: use Vec<EventTick> instead.
    pub events: Vec<serde_json::Value>,
    pub gotv_bots: Vec<String>,
    pub map: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mm_rank_update: Option<serde_json::Value>,
    pub player_names: HashMap<String, String>,
    pub player_slots: HashMap<String, i32>,
    pub servername: String,
    pub tickrate: f32,
}

impl Default for DemoInfo {
    fn default() -> Self {
        DemoInfo {
            events: Vec::new(),
            map: String::new(),
            gotv_bots: Vec::new(),
            mm_rank_update: None,
            player_names: Default::default(),
            player_slots: Default::default(),
            servername: String::new(),
            tickrate: 0.0,
        }
    }
}

#[derive(Serialize)]
pub struct EventTick {
    pub tick: Tick,
    #[serde(flatten)]
    pub event: Event,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    BombDefused(BombDefused),
    BombExploded(BombExploded),
    PlayerHurt(PlayerHurt),
    PlayerDeath(PlayerDeath),
    PlayerDisconnected(PlayerDisconnect),
    RoundStart(RoundStart),
    RoundEnd(RoundEnd),
    RoundOfficiallyEnded,
}

type Xuid = u64;

#[derive(Serialize)]
pub struct BombDefused {
    pub userid: Xuid,
}

#[derive(Serialize)]
pub struct BombExploded {
    pub userid: Xuid,
}

#[derive(Serialize)]
pub struct PlayerHurt {
    pub userid: Xuid,
    pub attacker: Xuid,
    pub dmg_health: i32,
}

#[derive(Serialize)]
pub struct PlayerDeath {
    pub userid: Xuid,
    pub attacker: Xuid,
    pub assister: Xuid,
    pub assistedflash: bool,
    pub weapon: String,
    pub headshot: bool,
    pub penetrated: i32,
    pub noscope: bool,
    pub thrusmoke: bool,
    pub attackerblind: bool,
    /// Distance in meters. 1 meter = 39.38 coordinate distance.
    pub distance: f32,
    /// Number of ticks since the attacker jumped. Only set if death occurred
    /// less than 0.75 seconds since the jump.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jump: Option<Tick>,
}

#[derive(Serialize)]
pub struct PlayerDisconnect {
    pub userid: Xuid,
}

#[derive(Serialize)]
pub struct RoundStart {
    pub timelimit: i32,
}

#[derive(Serialize)]
pub struct RoundEnd {
    pub winner: i32,
    pub reason: i32,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_event_json() -> anyhow::Result<()> {
        let defuse = EventTick {
            tick: 1,
            event: Event::BombDefused(BombDefused { userid: 2 }),
        };
        assert_eq!(
            serde_json::to_string(&defuse)?,
            r#"{"tick":1,"type":"bomb_defused","userid":2}"#
        );
        Ok(())
    }
}
