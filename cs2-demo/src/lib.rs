mod demo_command;
mod message;
mod packet;
pub mod proto;
mod send_tables;
mod string_table;

use crate::proto::demo::EDemoCommands;
use demo_format::Tick;
use protobuf::CodedInputStream;
use std::io;

pub use crate::demo_command::DemoCommand;
pub use crate::message::Message;
pub use crate::string_table::{PlayerInfo, StringTable, UserInfo};

/// Error type for this library.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Protobuf(#[from] protobuf::Error),
    #[error("invalid demo type (expected: PBDEMS2, found: {found})")]
    InvalidDemoType { found: String },
    #[error("unknown packet command found: {0}")]
    UnknownPacketCommand(u32),
    #[error(transparent)]
    Decompression(#[from] snap::Error),
    #[error("missing string_table from CDemoFullPacket")]
    MissingStringTable,
    #[error("missing packet from CDemoFullPacket")]
    MissingPacket,
    #[error("cannot parse string table player index")]
    InvalidPlayerIndex,
    #[error("cannot parse sendtables")]
    InvalidSendTables,
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct DemoParser<'a> {
    reader: CodedInputStream<'a>,
}

impl<'a> DemoParser<'a> {
    pub fn try_new_after_demo_type(read: &'a mut dyn io::Read) -> Result<Self> {
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
