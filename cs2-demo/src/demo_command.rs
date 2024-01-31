use protobuf::CodedInputStream;
use protobuf::Message;
use std::fmt;

use crate::packet::Packet;
use crate::proto::demo::{
    CDemoClassInfo, CDemoFileHeader, CDemoFullPacket, CDemoPacket, CDemoSendTables,
    CDemoStringTables, EDemoCommands,
};
use crate::Tick;
use crate::{Error, Result};

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
    SendTables(CDemoSendTables),
    ClassInfo(CDemoClassInfo),
    StringTables(CDemoStringTables),
    Packet(Packet),
    ConsoleCmd,
    CustomData,
    CustomDataCallbacks,
    UserCmd,
    FullPacket(CDemoStringTables, Packet),
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
            4 => DemoCommand::SendTables(CDemoSendTables::parse_from_bytes(data)?),
            5 => DemoCommand::ClassInfo(CDemoClassInfo::parse_from_bytes(data)?),
            6 => DemoCommand::StringTables(CDemoStringTables::parse_from_bytes(data)?),
            // SignonPacket seems to be identical to Packet.
            7 | 8 => DemoCommand::Packet(Packet::try_new(CDemoPacket::parse_from_bytes(data)?)?),
            9 => DemoCommand::ConsoleCmd,
            10 => DemoCommand::CustomData,
            11 => DemoCommand::CustomDataCallbacks,
            12 => DemoCommand::UserCmd,
            13 => {
                let mut fp = CDemoFullPacket::parse_from_bytes(data)?;
                let string_tables = fp.string_table.take().ok_or(Error::MissingStringTable)?;
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

pub struct DemoParser<'a> {
    reader: CodedInputStream<'a>,
}

impl<'a> DemoParser<'a> {
    pub fn try_new_after_demo_type(read: &'a mut dyn std::io::Read) -> Result<Self> {
        let mut reader = CodedInputStream::new(read);
        reader.skip_raw_bytes(8)?;
        Ok(Self { reader })
    }

    pub fn parse_next_demo_command(&mut self) -> Result<Option<(Tick, DemoCommand)>> {
        if self.reader.eof()? {
            return Ok(None);
        }
        let cmd_flags = self.reader.read_raw_varint32()?;
        let cmd = cmd_flags & !(EDemoCommands::DEM_IsCompressed as u32);
        let compressed = (cmd_flags & (EDemoCommands::DEM_IsCompressed as u32)) != 0;
        let tick = self.reader.read_raw_varint32()? as i32;
        let size = self.reader.read_raw_varint32()?;
        let data = self.reader.read_raw_bytes(size)?;
        let data = if compressed {
            snap::raw::Decoder::new().decompress_vec(data.as_slice())?
        } else {
            data
        };
        Ok(Some((tick, DemoCommand::try_new(cmd, &data)?)))
    }
}
