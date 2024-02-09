/// Error type for this library.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Protobuf(#[from] protobuf::Error),
    #[error("invalid demo type (expected: PBDEMS2, found: {0:?})")]
    InvalidDemoType(Box<[u8]>),
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
    #[error("invalid entity id in PacketEntities")]
    InvalidEntityId,
    #[error("missing class_id in CDemoClassInfo")]
    MissingClassId,
    #[error("missing class name CDemoClassInfo")]
    MissingClassName,
    #[error("skipped class_id in CDemoClassInfo")]
    SkippedClassId,
    #[error("duplicate serializer in CDemoSendTables")]
    DuplicateSerializer,
    #[error("PacketEntities before ClassInfo")]
    EntityBeforeClassInfo,
    #[error("ClassInfo before SendTables")]
    ClassInfoBeforeSendTables,
    #[error("Missing polymorphic type from {field}")]
    MissingPolymorphicType { field: String },
    #[error(transparent)]
    Visitor(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
