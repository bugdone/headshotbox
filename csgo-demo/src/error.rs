use std::io;

pub type Result<T> = std::result::Result<T, Error>;

/// Error when parsing the header of a demo file.
#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum HeaderParsingError {
    #[error("invalid demo type (expected: HL2DEMO, found: {0:?})")]
    InvalidDemoType(Box<[u8]>),
    #[error("invalid demo protocol (expected: {expected}, found: {found})")]
    InvalidDemoProtocol { expected: u32, found: u32 },
    #[error("invalid game (expected: {expected}, found: {found})")]
    InvalidGame {
        expected: &'static str,
        found: String,
    },
}

/// Error when parsing data tables of a demo file.
#[derive(thiserror::Error, Debug)]
pub enum DataTablesParsingError {
    #[error("expected a send table")]
    NotASendTable,
    #[error("no send table has been found for this server class: {0}")]
    NoAssociatedSendTableForServerClass(String),
    #[error("invalid server class index (expected: < {expected_max}, found: {found})")]
    InvalidServerClassIndex { expected_max: usize, found: usize },
}

/// Error type for this library.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Protobuf(#[from] protobuf::Error),
    #[error(transparent)]
    HeaderParsing(#[from] HeaderParsingError),
    #[error("unknown packet command found: {0}")]
    UnknownPacketCommand(u8),
    #[error(transparent)]
    DataTablesParsing(#[from] DataTablesParsingError),
    #[error("StringTable error: {0}")]
    StringTable(&'static str),
    #[error("Entity error: {0}")]
    Entity(&'static str),
    #[error("ServerClass error: {0}")]
    ServerClass(String),
}
