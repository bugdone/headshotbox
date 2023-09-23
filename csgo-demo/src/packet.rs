use protobuf::CodedInputStream;
use tracing::{instrument, trace};

use crate::error::Result;
use crate::message::Message;

#[derive(Debug)]
pub(crate) struct Packet {
    pub messages: Vec<Message>,
}

impl Packet {
    #[instrument(level = "trace", skip(reader))]
    pub(crate) fn try_new(reader: &mut CodedInputStream) -> Result<Self> {
        // Skip command header, it contains —supposedly— no useful information
        // CommandInfo (152 bytes), SeqNrIn (4 bytes) and SeqNrOut (4 bytes)
        reader.skip_raw_bytes(152 + 4 + 4)?;

        let mut messages: Vec<Message> = Vec::new();
        let end_position = reader.read_fixed32()? as u64 + reader.pos();

        while reader.pos() < end_position {
            let message = Message::try_new(reader)?;
            trace!(?message);
            messages.push(message);
        }

        Ok(Self { messages })
    }
}
