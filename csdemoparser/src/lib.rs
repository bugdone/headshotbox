mod entity;
mod geometry;
mod string_table;

use crate::entity::ServerClasses;
use crate::entity::{Entities, Entity, EntityId, PropValue, Scalar};
use crate::geometry::{through_smoke, Point};
use crate::string_table::{PlayerInfo, StringTables};
use anyhow::bail;
use csgo_demo_parser::messages::csvcmsg_game_event_list::Descriptor_t;
use csgo_demo_parser::messages::{CSVCMsg_GameEvent, CSVCMsg_GameEventList};
use csgo_demo_parser::parser::packet::message::{Message, SvcMessage, UsrMessage};
use csgo_demo_parser::parser::string_table::StringTable;
use csgo_demo_parser::parser::PacketContent;
use csgo_demo_parser::DemoParser;
use entity::TrackProp;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io;
use std::rc::Rc;
use tracing::instrument;

type BitReader<'a> = bitstream_io::BitReader<&'a [u8], bitstream_io::LittleEndian>;

const PLAYER_CLASS: &str = "CCSPlayer";
const VEC_ORIGIN_XY: &str = "m_vecOrigin";
const VEC_ORIGIN_Z: &str = "m_vecOrigin[2]";
const VEC_VELOCITY_Z: &str = "m_vecVelocity[2]";
const IS_SCOPED: &str = "m_bIsScoped";

const GAME_RULES_CLASS: &str = "CCSGameRulesProxy";
const GAME_RESTART: &str = "m_bGameRestart";

const TEAM_CLASS: &str = "CCSTeam";

type Tick = i32;

#[derive(Serialize, Deserialize, Clone)]
pub struct DemoInfo {
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

impl DemoInfo {
    fn new(servername: String) -> Self {
        DemoInfo {
            events: Vec::new(),
            map: String::new(),
            gotv_bots: Vec::new(),
            mm_rank_update: None,
            player_names: Default::default(),
            player_slots: Default::default(),
            servername,
            tickrate: 0.0,
        }
    }
}

pub fn parse(read: &mut dyn io::Read) -> anyhow::Result<DemoInfo> {
    let mut parser = DemoParser::try_new(read)?;
    let server_name = parser.header().server_name().to_string();
    let mut server_classes = None;
    let mut packets = vec![];
    while let Some((header, content)) = parser.parse_next_packet()? {
        match content {
            PacketContent::DataTables(dt) => {
                server_classes = Some(ServerClasses::try_new(dt)?);
                break;
            }
            PacketContent::Packet(pv) => packets.push((pv, *header.tick())),
            PacketContent::StringTables(_) => todo!(),
            _ => (),
        }
        if packets.len() > 1000 {
            bail!("no DataTables in the first 1000 packets")
        }
    }

    let Some(mut server_classes) = server_classes else { bail!("no data tables before the first event") };
    let mut hsbox = HeadshotBoxParser::new(server_name, &mut server_classes);
    for (pv, tick) in packets {
        for p in pv {
            hsbox.handle_packet(p, tick)?;
        }
    }
    while let Some((header, content)) = parser.parse_next_packet()? {
        match content {
            PacketContent::Packet(pv) => {
                for p in pv {
                    hsbox.handle_packet(p, *header.tick())?;
                }
            }
            PacketContent::StringTables(st) => hsbox.handle_string_tables(st)?,
            _ => (),
        }
    }
    hsbox.get_info()
}

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
        let Some(pos) = self.team_entity_id.iter().position(|i| &Some(entity.id) == i)
        else { return false };
        let &PropValue::Scalar(Scalar::I32(new_score)) = value else { return false };
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

type GameEvent = serde_json::Map<String, serde_json::Value>;

struct HeadshotBoxParser<'a> {
    eventlist_descriptors: HashMap<i32, Descriptor_t>,
    string_tables: StringTables,
    players: HashMap<i32, PlayerInfo>,
    jumped_last: HashMap<i32, Tick>,
    tick_interval: f32,
    entities: Entities<'a>,
    smokes: BTreeMap<u16, Point>,
    bot_takeover: HashMap<u64, i32>,
    scoped_since: Rc<RefCell<HashMap<u16, Tick>>>,
    score: Rc<RefCell<TeamScore>>,
    demoinfo: Rc<RefCell<DemoInfo>>,
}

impl<'a> HeadshotBoxParser<'a> {
    fn new(server_name: String, server_classes: &'a mut ServerClasses) -> Self {
        let scoped_since = Rc::new(RefCell::new(HashMap::new()));
        let score: Rc<RefCell<TeamScore>> = Rc::new(RefCell::new(Default::default()));
        let demoinfo = Rc::new(RefCell::new(DemoInfo::new(server_name)));

        for sc in server_classes.server_classes.iter_mut() {
            for prop in sc.props.iter_mut() {
                prop.track = match (sc.name.as_str(), prop.name.as_str()) {
                    (TEAM_CLASS, "m_iTeamNum") => {
                        let score = Rc::clone(&score);
                        TrackProp::Changes(Rc::new(move |entity, _, value| match value {
                            PropValue::Scalar(Scalar::I32(id)) if id == &2 || id == &3 => {
                                score.borrow_mut().team_entity_id[(id - 2) as usize]
                                    .replace(entity.id);
                            }
                            _ => (),
                        }))
                    }
                    (TEAM_CLASS, "m_scoreTotal") => {
                        let score = Rc::clone(&score);
                        let demoinfo = Rc::clone(&demoinfo);
                        TrackProp::Changes(Rc::new(move |entity, tick, value| {
                            let mut score = score.borrow_mut();
                            if score.update(entity, value) {
                                demoinfo.borrow_mut().events.push(json!({
                                    "type": "score_changed",
                                    "tick": tick,
                                    "score": [score.score[0], score.score[1]],
                                }));
                            }
                        }))
                    }
                    (GAME_RULES_CLASS, GAME_RESTART) => {
                        let demoinfo = Rc::clone(&demoinfo);
                        TrackProp::Changes(Rc::new(move |_, tick, value| {
                            if let PropValue::Scalar(Scalar::I32(1)) = value {
                                demoinfo
                                    .borrow_mut()
                                    .events
                                    .push(json!({"type": "game_restart", "tick": tick}));
                            }
                        }))
                    }
                    (PLAYER_CLASS, VEC_ORIGIN_XY | VEC_ORIGIN_Z | VEC_VELOCITY_Z) => {
                        TrackProp::Value
                    }
                    (PLAYER_CLASS, IS_SCOPED) => {
                        let scoped_since = Rc::clone(&scoped_since);
                        TrackProp::Changes(Rc::new(move |entity, tick, value| {
                            if let PropValue::Scalar(Scalar::I32(1)) = value {
                                scoped_since.borrow_mut().insert(entity.id, tick);
                            } else {
                                scoped_since.borrow_mut().remove(&entity.id);
                            }
                        }))
                    }
                    _ => TrackProp::No,
                }
            }
        }

        HeadshotBoxParser {
            eventlist_descriptors: Default::default(),
            string_tables: StringTables::new(),
            players: Default::default(),
            jumped_last: HashMap::new(),
            tick_interval: 0.0,
            entities: Entities::new(server_classes),
            smokes: Default::default(),
            bot_takeover: Default::default(),
            scoped_since,
            score,
            demoinfo,
        }
    }

    fn update_players(
        players: &mut HashMap<i32, PlayerInfo>,
        demoinfo: &Rc<RefCell<DemoInfo>>,
        player_info: PlayerInfo,
    ) {
        let mut demoinfo = demoinfo.borrow_mut();
        if !player_info.fakeplayer && !player_info.is_hltv {
            demoinfo
                .player_slots
                .insert(player_info.xuid.to_string(), player_info.entity_id);
            demoinfo
                .player_names
                .insert(player_info.xuid.to_string(), player_info.name.to_string());
        }
        players.retain(|_, p| p.entity_id != player_info.entity_id);
        players.insert(player_info.user_id, player_info);
    }

    fn handle_string_tables(&mut self, st: Vec<StringTable>) -> anyhow::Result<()> {
        // demoinfogo clears the players but I don't think this is correct
        self.players.clear();
        for st in st.iter().filter(|st| st.name() == "userinfo") {
            for (entity_id, string) in st.strings().iter().enumerate() {
                if let Some(data) = string.data() {
                    let player_info = string_table::parse_player_info(data, entity_id as i32)?;
                    Self::update_players(&mut self.players, &self.demoinfo, player_info);
                }
            }
        }
        Ok(())
    }

    #[instrument(skip_all)]
    fn handle_packet(&mut self, p: Message, tick: Tick) -> anyhow::Result<()> {
        match p {
            Message::Svc(SvcMessage::ServerInfo(info)) => {
                self.demoinfo.borrow_mut().map = info.map_name().to_string();
                self.tick_interval = info.tick_interval();
            }
            Message::Svc(SvcMessage::CreateStringTable(table)) => {
                let mut updates = self.string_tables.create_string_table(&table);
                while let Some(player_info) = updates.next()? {
                    Self::update_players(&mut self.players, &self.demoinfo, player_info);
                }
            }
            Message::Svc(SvcMessage::UpdateStringTable(table)) => {
                let mut updates = self.string_tables.update_string_table(&table)?;
                while let Some(player_info) = updates.next()? {
                    Self::update_players(&mut self.players, &self.demoinfo, player_info);
                }
            }
            Message::Svc(SvcMessage::GameEventList(el)) => {
                self.eventlist_descriptors = parse_game_event_list(el)
            }
            Message::Svc(SvcMessage::GameEvent(event)) => {
                if let Some(descriptor) = self.eventlist_descriptors.get(&event.eventid()) {
                    let attrs = self.event_map(event, descriptor, tick)?;
                    self.handle_game_event(attrs, tick)?;
                }
            }
            Message::Usr(UsrMessage::ServerRankUpdate(ranks)) => {
                let mut mm_rank_update = serde_json::Map::new();
                for update in ranks.rank_update {
                    let mut attr = serde_json::Map::new();
                    if update.has_num_wins() {
                        attr.insert("num_wins".to_string(), json!(update.num_wins()));
                    }
                    if update.has_rank_old() {
                        attr.insert("rank_old".to_string(), json!(update.rank_old()));
                    }
                    if update.has_rank_new() {
                        attr.insert("rank_new".to_string(), json!(update.rank_new()));
                    }
                    if update.has_rank_change() {
                        attr.insert("rank_change".to_string(), json!(update.rank_change()));
                    }
                    let xuid = account_id_to_xuid(update.account_id());
                    mm_rank_update.insert(xuid.to_string(), serde_json::Value::Object(attr));
                }
                self.demoinfo.borrow_mut().mm_rank_update =
                    Some(serde_json::Value::Object(mm_rank_update));
            }
            Message::Svc(SvcMessage::PacketEntities(msg)) => {
                self.entities.read_packet_entities(msg, tick)?
            }
            _ => (),
        }
        Ok(())
    }

    fn add_smoke(&mut self, attrs: &GameEvent) -> Option<()> {
        let entity_id = maybe_get_u16(attrs.get("entityid"))?;
        let x = attrs.get("x")?.as_f64()?;
        let y = attrs.get("y")?.as_f64()?;
        let z = attrs.get("z")?.as_f64()?;
        let p = Point::new(x, y, z);
        self.smokes.insert(entity_id, p);
        None
    }

    fn handle_game_event(&mut self, mut attrs: GameEvent, tick: Tick) -> anyhow::Result<()> {
        let emit = |attrs| {
            self.demoinfo
                .borrow_mut()
                .events
                .push(serde_json::Value::Object(attrs))
        };
        match attrs.get("type").unwrap().as_str().unwrap() {
            "player_jump" => {
                if let Some(user_id) = maybe_get_i32(attrs.get("userid")) {
                    self.jumped_last.insert(user_id, tick);
                }
            }
            "smokegrenade_detonate" => {
                self.add_smoke(&attrs);
            }
            "smokegrenade_expired" => {
                if let Some(entity_id) = maybe_get_u16(attrs.get("entityid")) {
                    self.smokes.remove(&entity_id);
                }
            }
            "round_start" => {
                self.smokes.clear();
                self.bot_takeover.clear();
                self.scoped_since.borrow_mut().clear();
                self.score.borrow_mut().set_round_start();
                emit(attrs);
            }
            "player_death" => {
                if let Some(attacker_user_id) = maybe_get_i32(attrs.get("attacker")) {
                    if self.players.get(&attacker_user_id).is_some() {
                        if let Some(jump) = self.jumped_last(attacker_user_id, tick) {
                            attrs.insert("jump".to_string(), json!(jump));
                        }
                    }
                }
                let attacker_info = self.get_player_info("attacker", &attrs);
                let victim = self.get_player_entity("userid", &attrs);
                let attacker = self.get_player_entity("attacker", &attrs);
                self.replace_user_id_with_xuid("userid", &mut attrs);
                self.replace_user_id_with_xuid("attacker", &mut attrs);
                self.replace_user_id_with_xuid("assister", &mut attrs);
                if let (Some(victim), Some(attacker)) = (victim, attacker) {
                    self.add_player_death_attrs(&mut attrs, victim, attacker);
                }
                if let Some(attacker) = attacker {
                    if let Some(PropValue::Scalar(Scalar::F32(z))) =
                        attacker.get_prop(VEC_VELOCITY_Z)
                    {
                        attrs.insert("air_velocity".into(), json!(z));
                    }
                    if let Some(since) = self.scoped_since.borrow().get(&attacker.id) {
                        if let Some(false) = attacker_info.map(|a| a.fakeplayer) {
                            attrs.insert("scoped_since".to_string(), json!(since));
                        }
                    }
                }
                emit(attrs);
            }
            "bot_takeover" => {
                if let Some(player_info) = self.get_player_info("userid", &attrs) {
                    if let Some(botid) = maybe_get_i32(attrs.get("botid")) {
                        self.bot_takeover.insert(player_info.xuid, botid);
                    }
                }
            }
            "player_connect" => {
                if let Some(player_info) = self.handle_player_connect(attrs) {
                    Self::update_players(&mut self.players, &self.demoinfo, player_info);
                }
            }
            "player_disconnect" => {
                let user_id = maybe_get_i32(attrs.get("userid"));
                self.replace_user_id_with_xuid("userid", &mut attrs);
                attrs.insert("type".to_string(), json!("player_disconnected"));
                attrs.remove("networkid");
                if let Some(user_id) = user_id {
                    self.players.remove(&user_id);
                }
                emit(attrs);
            }
            _ => {
                self.replace_user_id_with_xuid("userid", &mut attrs);
                self.replace_user_id_with_xuid("attacker", &mut attrs);
                emit(attrs);
            }
        }
        Ok(())
    }

    fn handle_player_connect(&self, attrs: GameEvent) -> Option<PlayerInfo> {
        let user_id = maybe_get_i32(attrs.get("userid"))?;
        let name = attrs.get("name")?.as_str()?.to_owned();
        let entity_id = maybe_get_i32(attrs.get("index"))?;
        let guid = attrs.get("networkid")?.as_str()?.to_string();
        let fakeplayer = guid == "BOT";
        let xuid = guid_to_xuid(&guid).unwrap_or(0);
        Some(PlayerInfo {
            version: 0,
            xuid,
            name,
            user_id,
            guid,
            friends_id: 0,
            friends_name: "".to_owned(),
            fakeplayer,
            is_hltv: false,
            files_downloaded: 0,
            entity_id,
        })
    }

    fn get_info(self) -> anyhow::Result<DemoInfo> {
        let mut demoinfo = self.demoinfo.borrow_mut();
        demoinfo.gotv_bots = self
            .players
            .values()
            .filter(|p| p.is_hltv)
            .map(|p| p.name.to_string())
            .collect();
        demoinfo.tickrate = self.tick_interval;
        // TODO: this is slow
        Ok((*demoinfo).clone())
    }

    fn event_map(
        &self,
        event: CSVCMsg_GameEvent,
        descriptor: &Descriptor_t,
        tick: Tick,
    ) -> anyhow::Result<GameEvent> {
        let mut attrs = serde_json::Map::new();
        for (i, descriptor_key) in descriptor.keys.iter().enumerate() {
            let event_key = &event.keys[i];
            let key = descriptor_key.name().to_string();
            if event_key.type_() != descriptor_key.type_() {
                bail!("event key type does not match descriptor type");
            }
            let val = match descriptor_key.type_() {
                1 => json!(event_key.val_string()),
                2 => json!(event_key.val_float()),
                3 => json!(event_key.val_long()),
                4 => json!(event_key.val_short()),
                5 => json!(event_key.val_byte()),
                6 => json!(event_key.val_bool()),
                7 => json!(event_key.val_uint64()),
                e => bail!("unknown event key type {}", e),
            };
            attrs.insert(key, val);
        }
        attrs.insert("type".into(), json!(descriptor.name()));
        attrs.insert("tick".into(), json!(tick));
        Ok(attrs)
    }

    fn jumped_last(&self, user_id: i32, tick: Tick) -> Option<Tick> {
        let &jumped_last = self.jumped_last.get(&user_id)?;
        const JUMP_DURATION: f64 = 0.75;
        if self.tick_interval > 0_f32
            && jumped_last as f64 >= tick as f64 - JUMP_DURATION / self.tick_interval as f64
        {
            Some(tick - jumped_last)
        } else {
            None
        }
    }

    fn get_player_info(&self, key: &str, attrs: &GameEvent) -> Option<&PlayerInfo> {
        let user_id = maybe_get_i32(attrs.get(key))?;
        let player_info = self.players.get(&user_id)?;
        Some(player_info)
    }

    fn get_player_entity(&self, key: &str, attrs: &GameEvent) -> Option<&Entity> {
        let user_id = maybe_get_i32(attrs.get(key))?;
        let player_info = self.players.get(&user_id)?;
        let entity_id: EntityId = player_info.entity_id as u16 + 1;
        self.entities.get(entity_id)
    }

    fn replace_user_id_with_xuid(&self, key: &str, attrs: &mut GameEvent) {
        if let Some(player_info) = self.get_player_info(key, attrs) {
            if player_info.fakeplayer {
                return;
            }
            let mut xuid = player_info.xuid;
            if let Some(&botid) = self.bot_takeover.get(&xuid) {
                match (attrs.get("type").and_then(serde_json::Value::as_str), key) {
                    // player_spawn happens before round_start when we clear bot_takeover.
                    (Some("player_spawn"), _) => {}
                    (Some("player_disconnect"), _) => {}
                    // CS:GO awards assists to the controlling human instead of the bot.
                    (Some("player_death"), "assister") => {}
                    _ => xuid = botid as u64,
                }
            }
            attrs.insert(key.to_string(), json!(xuid));
        }
    }

    fn add_player_death_attrs(&self, attrs: &mut GameEvent, victim: &Entity, attacker: &Entity) {
        if let (Some(victim_pos), Some(attacker_pos)) =
            (self.get_position(victim), self.get_position(attacker))
        {
            let smokes: Vec<serde_json::Value> = self
                .smokes
                .values()
                .filter(|smoke| through_smoke(&attacker_pos, &victim_pos, smoke))
                .map(|smoke| (*smoke).into())
                .collect();
            if !smokes.is_empty() {
                attrs.insert("smoke".into(), json!(smokes));
            }
            attrs.insert("attacker_pos".into(), attacker_pos.into());
            attrs.insert("victim_pos".into(), victim_pos.into());
        }
    }

    fn get_position(&self, entity: &Entity) -> Option<Point> {
        if let PropValue::Scalar(Scalar::Vector(xy)) = entity.get_prop(VEC_ORIGIN_XY)? {
            if let PropValue::Scalar(Scalar::F32(z)) = entity.get_prop(VEC_ORIGIN_Z)? {
                return Some(Point::new(xy.x as f64, xy.y as f64, *z as f64));
            }
        }
        None
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

fn parse_game_event_list(el: CSVCMsg_GameEventList) -> HashMap<i32, Descriptor_t> {
    let mut eventlist_descriptors = HashMap::new();
    let hsbox_events: HashSet<&str> = HashSet::from([
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
    for descriptor in el
        .descriptors
        .iter()
        .filter(|d| hsbox_events.contains(d.name()))
    {
        eventlist_descriptors.insert(descriptor.eventid(), descriptor.clone());
    }
    eventlist_descriptors
}

// Number of  bits needed to represent values in the 0..=n interval.
fn num_bits(n: u32) -> u32 {
    if n == 0 {
        1
    } else {
        u32::BITS - n.leading_zeros()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_json_diff::assert_json_eq;

    fn make_parser(server_classes: &mut ServerClasses) -> HeadshotBoxParser {
        let mut parser = HeadshotBoxParser::new("".to_owned(), server_classes);
        parser.tick_interval = 1f32 / 64f32;
        parser.players.insert(
            7,
            PlayerInfo {
                xuid: 1007,
                ..Default::default()
            },
        );
        parser
    }

    fn make_server_classes() -> ServerClasses {
        ServerClasses {
            bits: 0,
            server_classes: vec![],
        }
    }

    fn make_game_event(object: serde_json::Value) -> GameEvent {
        object.as_object().unwrap().clone()
    }

    fn emitted_event(
        parser: &mut HeadshotBoxParser,
        event: serde_json::Value,
        tick: Tick,
        expected: serde_json::Value,
    ) {
        handle_event(parser, event, tick);
        assert_json_eq!(parser.demoinfo.borrow().events.last().unwrap(), expected);
    }

    fn handle_event(parser: &mut HeadshotBoxParser, event: serde_json::Value, tick: Tick) {
        let attrs = make_game_event(event);
        parser.handle_game_event(attrs, tick).unwrap()
    }

    #[test]
    fn jump_death() {
        let mut server_classes = make_server_classes();
        let mut parser = make_parser(&mut server_classes);
        handle_event(&mut parser, json!({"type": "player_jump", "userid": 7}), 1);
        emitted_event(
            &mut parser,
            json!({"type": "player_death", "userid": 7, "attacker": 7}),
            2,
            json!({"type": "player_death", "userid": 1007, "attacker": 1007, "jump": 1}),
        );
    }

    #[test]
    fn jump_disconnect_death() {
        let mut server_classes = make_server_classes();
        let mut parser = make_parser(&mut server_classes);
        handle_event(&mut parser, json!({"type": "player_jump", "userid": 7}), 1);
        handle_event(
            &mut parser,
            json!({"type": "player_disconnect", "userid": 7}),
            2,
        );
        emitted_event(
            &mut parser,
            json!({"type": "player_death", "userid": 7,  "attacker": 7}),
            2,
            json!({"type": "player_death", "userid": 7, "attacker": 7}),
        );
    }

    #[test]
    fn bot_takeover() {
        let mut server_classes = make_server_classes();
        let mut parser = make_parser(&mut server_classes);
        handle_event(
            &mut parser,
            json!({"type": "bot_takeover", "userid": 7, "botid": 31}),
            1,
        );
        emitted_event(
            &mut parser,
            json!({"type": "player_hurt", "attacker": 7}),
            2,
            json!({"type": "player_hurt", "attacker": 31}),
        );
        emitted_event(
            &mut parser,
            json!({"type": "player_death", "attacker": 7}),
            2,
            json!({"type": "player_death", "attacker": 31}),
        );
        emitted_event(
            &mut parser,
            json!({"type": "player_death", "assister": 7}),
            2,
            json!({"type": "player_death", "assister": 1007}),
        );
        emitted_event(
            &mut parser,
            json!({"type": "player_spawn", "userid": 7}),
            3,
            json!({"type": "player_spawn", "userid": 1007}),
        );
        emitted_event(
            &mut parser,
            json!({"type": "player_disconnect", "userid": 7}),
            3,
            json!({"type": "player_disconnected", "userid": 1007}),
        );
    }

    #[test]
    fn test_guid_to_xuid() {
        assert_eq!(
            guid_to_xuid("STEAM_1:0:30828430").unwrap(),
            76561198021922588
        );
        assert!(guid_to_xuid("BOT").is_err());
    }
}
