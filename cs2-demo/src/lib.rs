mod demo_command;
pub mod entity;
mod error;
pub mod game_event;
mod message;
mod packet;
pub mod proto;
mod string_table;
#[cfg(test)]
mod testdata;
mod visit;

pub use crate::error::{Error, Result};
pub use crate::game_event::GameEventDescriptors;
pub use crate::string_table::{PlayerInfo, StringTable, UserInfo};
pub use crate::visit::{parse_after_demo_type, Visitor};

#[allow(dead_code)]
pub(crate) fn dump<M>(msg: &M, file: &str)
where
    M: protobuf::Message,
{
    let mut out = std::fs::File::create(file).unwrap();
    let mut os = protobuf::CodedOutputStream::new(&mut out);
    msg.write_to(&mut os).unwrap();
    os.flush().unwrap();
}
