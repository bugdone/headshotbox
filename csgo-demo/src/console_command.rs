use crate::Result;
use demo_format::read::ReadExt;
use protobuf::CodedInputStream;

#[derive(Debug)]
pub(crate) struct ConsoleCommand {
    pub command: String,
}

impl ConsoleCommand {
    pub(crate) fn try_new(reader: &mut CodedInputStream) -> Result<Self> {
        let size = reader.read_fixed32()?;
        let command = reader.read_string_limited(size as usize)?;
        Ok(Self { command })
    }
}
