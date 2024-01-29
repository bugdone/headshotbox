#![allow(dead_code)]

use serde::Deserialize;

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
