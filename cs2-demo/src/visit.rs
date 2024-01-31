use std::io::Read;

use tracing::{trace, trace_span};

use crate::demo_command::{DemoCommand, DemoParser};
use crate::entity::{Classes, Entities, SendTables};
use crate::game_event::{parse_game_event_list, GameEventDescriptors};
use crate::message::Message;
use crate::packet::Packet;
use crate::proto::demo::{CDemoFileHeader, CDemoStringTables};
use crate::proto::gameevents::CMsgSource1LegacyGameEvent;
use crate::proto::netmessages::CSVCMsg_ServerInfo;
use crate::string_table::{
    parse_create_string_table, parse_update_string_table, parse_userinfo, StringTableData,
    StringTableInfo, INSTANCEBASELINE, USERINFO,
};
use crate::{Error, Result, Tick, UserInfo};

pub trait Visitor {
    fn visit_file_header(&mut self, _file_header: CDemoFileHeader) -> anyhow::Result<()> {
        Ok(())
    }
    fn visit_server_info(&mut self, _server_info: CSVCMsg_ServerInfo) -> anyhow::Result<()> {
        Ok(())
    }
    fn visit_userinfo_table(&mut self, _user_info: Vec<UserInfo>) -> anyhow::Result<()> {
        Ok(())
    }
    fn visit_game_event(
        &mut self,
        _game_event: CMsgSource1LegacyGameEvent,
        _tick: Tick,
        _entities: &Entities,
    ) -> anyhow::Result<()> {
        Ok(())
    }
    fn visit_game_event_descriptors(
        &mut self,
        _descriptors: GameEventDescriptors,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

pub fn parse_after_demo_type(read: &mut dyn Read, visitor: &mut dyn Visitor) -> Result<()> {
    DemoVisit::new(DemoParser::try_new_after_demo_type(read)?, visitor).parse()
}

struct DemoVisit<'a> {
    parser: DemoParser<'a>,
    visitor: &'a mut dyn Visitor,
    send_tables: Option<SendTables>,
    classes: Option<Classes>,
    // Information necessary to parse UpdateStringTable.
    string_tables: Vec<Option<StringTableInfo>>,
    // Data from "instancebaselines" string table cached until `classes` gets created.
    instance_baselines: StringTableData,
    entities: Entities,
}

impl<'a> DemoVisit<'a> {
    fn new(parser: DemoParser<'a>, visitor: &'a mut dyn Visitor) -> Self {
        Self {
            parser,
            visitor,
            send_tables: None,
            classes: None,
            string_tables: Default::default(),
            instance_baselines: Default::default(),
            entities: Default::default(),
        }
    }
}

impl DemoVisit<'_> {
    fn parse(&mut self) -> Result<()> {
        while let Some((tick, cmd)) = self.parser.parse_next_demo_command()? {
            trace_span!("demo_command").in_scope(|| trace!("#{tick} {cmd}"));
            match cmd {
                DemoCommand::FileHeader(header) => self.visitor.visit_file_header(header)?,
                DemoCommand::SendTables(send) => {
                    self.send_tables = Some(SendTables::try_new(send)?)
                }
                DemoCommand::ClassInfo(ci) => {
                    let send_tables = self
                        .send_tables
                        .take()
                        .ok_or(Error::ClassInfoBeforeSendTables)?;
                    let mut classes = Classes::try_new(ci, send_tables)?;
                    classes.update_instance_baselines(self.instance_baselines.split_off(0));
                    self.classes = Some(classes);
                }
                DemoCommand::StringTables(st) => self.parse_string_tables(st)?,
                DemoCommand::Packet(p) => self.parse_packet(tick, p)?,
                DemoCommand::FullPacket(st, p) => {
                    self.parse_string_tables(st)?;
                    self.parse_packet(tick, p)?
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn parse_string_tables(&mut self, st: CDemoStringTables) -> Result<()> {
        for mut table in st.tables {
            let name = table.take_table_name();
            match name.as_str() {
                USERINFO => self.visitor.visit_userinfo_table(parse_userinfo(&table)?)?,
                INSTANCEBASELINE => {
                    let data = table
                        .items
                        .into_iter()
                        .map(|mut e| (e.take_str(), e.take_data()))
                        .collect();
                    self.update_instance_baselines(data)?;
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn update_instance_baselines(&mut self, items: StringTableData) -> Result<()> {
        match self.classes.as_mut() {
            Some(classes) => {
                assert!(self.instance_baselines.is_empty());
                classes.update_instance_baselines(items);
            }
            None => self.instance_baselines.extend(items),
        }
        Ok(())
    }

    fn parse_packet(&mut self, tick: i32, p: Packet) -> Result<()> {
        for msg in p.messages {
            match msg {
                Message::PacketEntities(pe) => {
                    let classes = self.classes.as_ref().ok_or(Error::EntityBeforeClassInfo)?;
                    self.entities.read_packet_entities(pe, classes)?
                }
                Message::ServerInfo(si) => self.visitor.visit_server_info(si)?,
                Message::Source1LegacyGameEventList(gel) => self
                    .visitor
                    .visit_game_event_descriptors(parse_game_event_list(gel))?,
                Message::Source1LegacyGameEvent(ge) => {
                    self.visitor.visit_game_event(ge, tick, &self.entities)?
                }
                Message::ClearAllStringTables(_) => self.string_tables.clear(),
                Message::CreateStringTable(msg) => {
                    let info = match msg.name() {
                        INSTANCEBASELINE => {
                            let (info, data) = parse_create_string_table(msg)?;
                            self.update_instance_baselines(data)?;
                            Some(info)
                        }
                        USERINFO => {
                            // TODO
                            None
                        }
                        _ => None,
                    };
                    self.string_tables.push(info);
                }
                Message::UpdateStringTable(msg) => {
                    if let Some(info) = self.string_tables[msg.table_id() as usize].as_ref() {
                        match info.name.as_str() {
                            INSTANCEBASELINE => {
                                let data = parse_update_string_table(msg, info)?;
                                self.update_instance_baselines(data)?;
                            }
                            USERINFO => { /* TODO */ }
                            _ => {}
                        }
                    }
                }
                Message::Unknown(_) => (),
            }
        }
        Ok(())
    }
}
