use crate::message::Message;
use crate::proto::demo::CDemoPacket;
use crate::Result;
use std::fmt;
use std::io::Cursor;

pub struct Packet {
    pub messages: Vec<Message>,
}

impl Packet {
    pub(crate) fn try_new(packet: CDemoPacket) -> Result<Self> {
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
