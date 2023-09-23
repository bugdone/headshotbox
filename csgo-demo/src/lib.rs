mod command;
mod console_command;
mod data_table;
mod error;
mod header;
mod message;
mod packet;
pub mod proto;
mod read_to_terminator;
mod string_table;
mod user_command;

use crate::command::{Command, PacketHeader};
use crate::console_command::ConsoleCommand;
use crate::packet::Packet;
use crate::string_table::StringTables;
use crate::user_command::UserCommandCompressed;
use getset::Getters;
use protobuf::CodedInputStream;
use std::io;
use tracing::trace;

pub use command::PacketContent;
pub use data_table::DataTables;
pub use error::{Error, Result};
pub use header::DemoHeader;
pub use message::Message;
pub use string_table::StringTable;

#[derive(Getters, Debug)]
#[getset(get = "pub")]
pub struct DemoParser<'a> {
    #[getset(skip)]
    reader: CodedInputStream<'a>,
    header: DemoHeader,
}

impl<'a> DemoParser<'a> {
    pub fn try_new_after_demo_type(read: &'a mut dyn io::Read) -> Result<Self> {
        let mut reader = CodedInputStream::new(read);
        let header = DemoHeader::try_new_after_demo_type(&mut reader)?;
        trace!(?header);

        Ok(Self { header, reader })
    }

    pub fn parse_next_packet(&mut self) -> Result<Option<(PacketHeader, PacketContent)>> {
        if self.reader.eof()? {
            return Ok(None);
        }

        let header = PacketHeader::try_new(&mut self.reader)?;
        trace!(?header);

        Ok(match header.command {
            Command::Stop => Some((header, PacketContent::Stop)),
            Command::SyncTick => Some((header, PacketContent::SyncTick)),
            Command::ConsoleCommand => {
                let console_command = ConsoleCommand::try_new(&mut self.reader)?;
                trace!(?console_command);
                Some((
                    header,
                    PacketContent::ConsoleCommand(console_command.command),
                ))
            }
            Command::UserCommand => {
                let user_command = UserCommandCompressed::try_new(&mut self.reader)?;
                trace!(?user_command);
                Some((header, PacketContent::UserCommand(user_command)))
            }
            Command::Packet | Command::Signon => {
                let packet = Packet::try_new(&mut self.reader)?;
                Some((header, PacketContent::Packet(packet.messages)))
            }
            Command::StringTables => {
                let string_tables = StringTables::try_new(&mut self.reader)?;
                Some((header, PacketContent::StringTables(string_tables.tables)))
            }
            Command::DataTables => {
                let data_tables = DataTables::try_new(&mut self.reader)?;
                Some((header, PacketContent::DataTables(data_tables)))
            }
            Command::CustomData => unimplemented!("custom data found"),
        })
    }
}
