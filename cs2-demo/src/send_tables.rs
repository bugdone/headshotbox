use crate::proto::demo::CDemoSendTables;
use crate::proto::netmessages::CSVCMsg_FlattenedSerializer;
use crate::{Error, Result};
use protobuf::CodedInputStream;

#[derive(Debug)]
pub struct SendTables {
    _flattened_serializer: CSVCMsg_FlattenedSerializer,
}

impl SendTables {
    pub(crate) fn try_new(msg: CDemoSendTables) -> Result<Self> {
        let mut data = msg.data();
        let _flattened_serializer: CSVCMsg_FlattenedSerializer = CodedInputStream::new(&mut data)
            .read_message()
            .or(Err(Error::InvalidSendTables))?;
        Ok(Self {
            _flattened_serializer,
        })
    }
}

#[cfg(test)]
mod tests {
    use protobuf::Message;

    use super::*;

    #[test]
    fn test() {
        let binpb = include_bytes!("testdata/cdemosendtables.binpb");
        let dst = CDemoSendTables::parse_from_bytes(binpb).unwrap();
        SendTables::try_new(dst).unwrap();
    }
}
