use protobuf::Message;

use crate::proto::demo::{CDemoClassInfo, CDemoSendTables, CDemoStringTables};
use crate::proto::netmessages::{
    CSVCMsg_CreateStringTable, CSVCMsg_PacketEntities, CSVCMsg_UpdateStringTable,
};

pub(crate) fn send_tables() -> CDemoSendTables {
    CDemoSendTables::parse_from_bytes(include_bytes!("testdata/cdemosendtables.binpb")).unwrap()
}

pub(crate) fn string_tables() -> CDemoStringTables {
    CDemoStringTables::parse_from_bytes(include_bytes!("testdata/CDemoStringTables.binpb")).unwrap()
}

pub(crate) fn class_info() -> CDemoClassInfo {
    CDemoClassInfo::parse_from_bytes(include_bytes!("testdata/cdemoclassinfo.binpb")).unwrap()
}

pub(crate) fn packet_entities() -> CSVCMsg_PacketEntities {
    CSVCMsg_PacketEntities::parse_from_bytes(include_bytes!("testdata/packetentities.binpb"))
        .unwrap()
}

pub(crate) fn create_string_table() -> CSVCMsg_CreateStringTable {
    CSVCMsg_CreateStringTable::parse_from_bytes(include_bytes!(
        "testdata/CSVCMsg_CreateStringTable.binpb"
    ))
    .unwrap()
}

pub(crate) fn update_string_table() -> crate::proto::netmessages::CSVCMsg_UpdateStringTable {
    CSVCMsg_UpdateStringTable::parse_from_bytes(include_bytes!(
        "testdata/CSVCMsg_UpdateStringTable.binpb"
    ))
    .unwrap()
}
