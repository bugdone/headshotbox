use demo_format::Tick;
use tracing::{trace, trace_span};

use crate::demo_command::{DemoCommand, DemoParser};
use crate::entity::{Classes, Entities, SendTables};
use crate::game_event::{parse_game_event_list, GameEventDescriptors};
use crate::message::Message;
use crate::proto::demo::CDemoFileHeader;
use crate::proto::gameevents::CMsgSource1LegacyGameEvent;
use crate::proto::netmessages::CSVCMsg_ServerInfo;
use crate::{Error, StringTable};

pub trait Visitor {
    fn visit_file_header(&mut self, _file_header: CDemoFileHeader) -> anyhow::Result<()> {
        Ok(())
    }
    fn visit_server_info(&mut self, _server_info: CSVCMsg_ServerInfo) -> anyhow::Result<()> {
        Ok(())
    }
    fn visit_string_tables(&mut self, _string_tables: Vec<StringTable>) -> anyhow::Result<()> {
        Ok(())
    }
    fn visit_game_event(
        &mut self,
        _game_event: CMsgSource1LegacyGameEvent,
        _tick: Tick,
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

pub fn parse_after_demo_type(
    read: &mut dyn std::io::Read,
    visitor: &mut dyn Visitor,
) -> anyhow::Result<()> {
    let mut parser = DemoParser::try_new_after_demo_type(read)?;
    let mut send_tables = None;
    let mut classes = None;
    let mut next = parser.parse_next_demo_command()?;
    while let Some((tick @ -1, cmd)) = next {
        trace_span!("demo_command").in_scope(|| trace!("#{tick} {cmd}"));
        match cmd {
            DemoCommand::SendTables(send) => send_tables = Some(SendTables::try_new(send)?),
            DemoCommand::ClassInfo(ci) => {
                let send_tables = send_tables.take().ok_or(Error::PacketOutOfOrder)?;
                classes = Some(Classes::try_new(ci, send_tables)?)
            }
            DemoCommand::Packet(p) => {
                for msg in p.messages {
                    match msg {
                        Message::PacketEntities(_) => Err(Error::PacketOutOfOrder)?,
                        Message::Unknown(_) => (),
                        _ => process_message(tick, msg, visitor)?,
                    }
                }
            }
            _ => process_demo_command(cmd, visitor)?,
        }
        next = parser.parse_next_demo_command()?;
    }

    let classes = classes.ok_or(Error::PacketOutOfOrder)?;
    let mut entities = Entities::default();
    while let Some((tick, cmd)) = next {
        trace_span!("demo_command").in_scope(|| trace!("#{tick:?} {cmd}"));
        match cmd {
            DemoCommand::SendTables(_) | DemoCommand::ClassInfo(_) => Err(Error::PacketOutOfOrder)?,
            DemoCommand::Packet(p) => {
                for msg in p.messages {
                    match msg {
                        Message::PacketEntities(pe) => {
                            entities.read_packet_entities(pe, &classes)?
                        }
                        Message::Unknown(_) => (),
                        _ => process_message(tick, msg, visitor)?,
                    }
                }
            }
            DemoCommand::StringTables(st) => visitor.visit_string_tables(st)?,
            _ => process_demo_command(cmd, visitor)?,
        }
        next = parser.parse_next_demo_command()?;
    }
    Ok(())
}

fn process_demo_command(cmd: DemoCommand, visitor: &mut dyn Visitor) -> anyhow::Result<()> {
    match cmd {
        DemoCommand::FileHeader(header) => visitor.visit_file_header(header),
        DemoCommand::StringTables(st) => visitor.visit_string_tables(st),
        _ => Ok(()),
    }
}

fn process_message(tick: Tick, msg: Message, visitor: &mut dyn Visitor) -> anyhow::Result<()> {
    match msg {
        Message::ServerInfo(si) => visitor.visit_server_info(si),
        Message::Source1LegacyGameEventList(gel) => {
            visitor.visit_game_event_descriptors(parse_game_event_list(gel))
        }
        Message::Source1LegacyGameEvent(ge) => visitor.visit_game_event(ge, tick),
        Message::PacketEntities(_) => unreachable!(),
        Message::Unknown(_) => Ok(()),
    }
}
