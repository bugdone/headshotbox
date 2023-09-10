use super::proto::demo::CDemoPacket;
use super::proto::gameevents::{
    CMsgSource1LegacyGameEvent, CMsgSource1LegacyGameEventList, EBaseGameEvents,
};
use super::Result;
use bitstream_io::BitRead;
use demo_format::read::ValveBitReader;
use paste::paste;
use protobuf::Message as protobuf_Message;
use std::fmt;
use std::io::{Cursor, Read, Seek, SeekFrom};

/// Generates an enum with a variant for each supported Packet message type.
///
/// $enum is a proto enum listing the Packet message identifiers for a category of messages
/// $enum_prefix is the prefix for all the items in $enum
/// $msg_prefix is the prefix for proto message type names
/// $name is the proto message type name without $msg_prefix
macro_rules! create_message_impl {
    ($(
        ($enum:ident, $enum_prefix:ident, $msg_prefix:ident)
        => [ $($name:ident),* ]
    ),*) => {paste! {
        pub enum Message {
            Unknown(u32),
            $($($name([<$msg_prefix $name>])),*),*
        }

        impl Message {
            pub(crate) fn try_new<R: Read + Seek>(
                reader: &mut bitstream_io::BitReader<R, bitstream_io::LittleEndian>,
                mut buffer: &mut Vec<u8>,
            ) -> Result<Message> {
                $($(const [<$name:upper>]: u32 = $enum::[<$enum_prefix _ $name>] as u32;)*)*
                let msg_type = reader.read_ubitvar()?;
                let size = reader.read_varint32()? as usize;
                match msg_type {
                    $($(
                        [<$name:upper>] => {
                            let msg = [<$msg_prefix $name>]::parse_from_bytes(read_buffer(
                                &mut buffer, size, reader)?)?;
                            Ok(Message::$name(msg))
                        }
                    )*)*
                    _ => {
                        reader.seek_bits(SeekFrom::Current(size as i64 * 8))?;
                        Ok(Message::Unknown(msg_type))
                    }
                }
            }
        }

        impl fmt::Debug for Message {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    Message::Unknown(t) => write!(f, "Unknown({})", t),
                    $($(Message::$name(m) => write!(f, "{}({})", stringify!($name), m),)*)*
                }
            }
        }
    }};
}

create_message_impl! {
    (EBaseGameEvents, GE, CMsg) => [
        Source1LegacyGameEvent,
        Source1LegacyGameEventList
    ]
}

pub struct Packet {
    pub messages: Vec<Message>,
}

impl Packet {
    pub fn try_new(packet: CDemoPacket) -> Result<Self> {
        let mut buffer = Vec::with_capacity(1024);
        let mut messages = Vec::new();
        let mut reader = bitstream_io::BitReader::new(Cursor::new(packet.data()));
        while reader.position_in_bits()? < packet.data().len() as u64 * 8 - 7 {
            messages.push(Message::try_new(&mut reader, &mut buffer)?);
        }
        Ok(Packet { messages })
    }
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.messages.is_empty() {
            writeln!(f, "Packet [")?;
            for msg in &self.messages {
                writeln!(f, "  {:?}", msg)?;
            }
            write!(f, "]")
        } else {
            write!(f, "Packet []")
        }
    }
}

fn read_buffer<'a, R: Read + Seek>(
    buffer: &'a mut Vec<u8>,
    size: usize,
    reader: &mut bitstream_io::BitReader<R, bitstream_io::LittleEndian>,
) -> Result<&'a [u8]> {
    if buffer.len() < size {
        buffer.resize(size, 0);
    }
    reader.read_bytes(&mut buffer[0..size])?;
    Ok(&buffer[0..size])
}
