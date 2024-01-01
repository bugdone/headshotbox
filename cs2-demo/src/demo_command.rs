use super::packet::Packet;
use super::proto::demo::{CDemoFileHeader, CDemoPacket, CDemoSendTables};
use super::send_tables::SendTables;
use super::{Error, Result};
use crate::proto::demo::{CDemoFullPacket, CDemoStringTables};
use crate::string_table::{parse_string_tables, StringTable};
use protobuf::Message;
use std::fmt;

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum DemoCommand {
    /// The last command.
    Stop,
    /// The first command.
    FileHeader(CDemoFileHeader),
    FileInfo,
    /// A sync tick. It contains no data.
    SyncTick,
    SendTables(SendTables),
    ClassInfo,
    StringTables(Vec<StringTable>),
    Packet(Packet),
    ConsoleCmd,
    CustomData,
    CustomDataCallbacks,
    UserCmd,
    FullPacket(Vec<StringTable>, Packet),
    SaveGame,
    SpawnGroups,
    AnimationData,
}

impl DemoCommand {
    pub fn try_new(cmd: u32, data: &[u8]) -> Result<Self> {
        let content = match cmd {
            0 => DemoCommand::Stop,
            1 => DemoCommand::FileHeader(CDemoFileHeader::parse_from_bytes(data)?),
            2 => DemoCommand::FileInfo,
            3 => DemoCommand::SyncTick,
            4 => DemoCommand::SendTables(SendTables::try_new(CDemoSendTables::parse_from_bytes(
                data,
            )?)?),
            5 => DemoCommand::ClassInfo,
            6 => DemoCommand::StringTables(parse_string_tables(
                CDemoStringTables::parse_from_bytes(data)?,
            )?),
            // SignonPacket seems to be identical to Packet.
            7 | 8 => DemoCommand::Packet(Packet::try_new(CDemoPacket::parse_from_bytes(data)?)?),
            9 => DemoCommand::ConsoleCmd,
            10 => DemoCommand::CustomData,
            11 => DemoCommand::CustomDataCallbacks,
            12 => DemoCommand::UserCmd,
            13 => {
                let mut fp = CDemoFullPacket::parse_from_bytes(data)?;
                let string_tables =
                    parse_string_tables(fp.string_table.take().ok_or(Error::MissingStringTable)?)?;
                let packet = Packet::try_new(fp.packet.take().ok_or(Error::MissingPacket)?)?;
                DemoCommand::FullPacket(string_tables, packet)
            }
            14 => DemoCommand::SaveGame,
            15 => DemoCommand::SpawnGroups,
            16 => DemoCommand::AnimationData,
            _ => return Err(Error::UnknownPacketCommand(cmd)),
        };
        Ok(content)
    }
}

impl fmt::Display for DemoCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DemoCommand::FileHeader(m) => write!(f, "FileHeader {}", m),
            DemoCommand::SendTables(_) => write!(f, "SendTables"),
            _ => write!(f, "{:?}", self),
        }
    }
}
