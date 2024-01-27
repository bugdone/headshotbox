use protobuf::Message;

use crate::proto::demo::{CDemoClassInfo, CDemoSendTables};
use crate::proto::netmessages::CSVCMsg_PacketEntities;

pub(crate) fn send_tables() -> CDemoSendTables {
    CDemoSendTables::parse_from_bytes(include_bytes!("testdata/cdemosendtables.binpb")).unwrap()
}

pub(crate) fn class_info() -> CDemoClassInfo {
    CDemoClassInfo::parse_from_bytes(include_bytes!("testdata/cdemoclassinfo.binpb")).unwrap()
}

pub(crate) fn packet_entities() -> CSVCMsg_PacketEntities {
    CSVCMsg_PacketEntities::parse_from_bytes(include_bytes!("testdata/packetentities.binpb"))
        .unwrap()
}
