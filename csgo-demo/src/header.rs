use getset::Getters;
use protobuf::CodedInputStream;

use crate::error::{HeaderParsingError, Result};
use crate::read::ReadExt;

const MAX_OS_PATH: usize = 260;

/// Expected demo type.
const EXPECTED_DEMO_TYPE: &[u8; 8] = b"HL2DEMO\0";
/// Expected demo protocol.
const EXPECTED_DEMO_PROTOCOL: u32 = 4;
/// Expected game name.
const EXPECTED_GAME: &str = "csgo"; // in lowercase

/// Header of a demo file. It contains general information about the demo.
#[derive(Getters, Debug)]
#[getset(get = "pub")]
pub struct DemoHeader {
    /// Demo protocol version. Should always be `4`.
    demo_protocol: u32,
    /// Network protocol version.
    network_protocol: u32,
    /// Name of the server on which the game was played.
    server_name: String,
    /// Name of the client. _Almost_ always `GOTV Demo`.
    client_name: String,
    /// Name of the map on which the game was played.
    map_name: String,
    /// Name of the game. Should always be `csgo`.
    game: String,
    /// Duration of the game, in seconds.
    duration: f32,
    /// Total number of ticks.
    ticks: u32,
    /// Total number of frames.
    frames: u32,
    /// Length of Signon data, in bytes.
    signon_length: u32,
}

impl DemoHeader {
    /// Assumes the demo type has already been read and is valid.
    pub(crate) fn try_new(reader: &mut CodedInputStream) -> Result<Self> {
        let mut demo_type = [std::mem::MaybeUninit::<u8>::uninit(); 8];
        reader.read_exact(&mut demo_type)?;
        let demo_type = unsafe { std::mem::transmute::<_, [u8; 8]>(demo_type) };
        if &demo_type != EXPECTED_DEMO_TYPE {
            Err(HeaderParsingError::InvalidDemoType(Box::new(demo_type)))?
        }
        let demo_protocol = reader.read_fixed32()?;
        if demo_protocol != EXPECTED_DEMO_PROTOCOL {
            Err(HeaderParsingError::InvalidDemoProtocol {
                expected: EXPECTED_DEMO_PROTOCOL,
                found: demo_protocol,
            })?;
        }

        let network_protocol = reader.read_fixed32()?;
        let server_name = reader.read_string_limited(MAX_OS_PATH)?;
        let client_name = reader.read_string_limited(MAX_OS_PATH)?;
        let map_name = reader.read_string_limited(MAX_OS_PATH)?;

        let game = reader.read_string_limited(MAX_OS_PATH)?;
        if game != EXPECTED_GAME {
            return Err(HeaderParsingError::InvalidGame {
                expected: EXPECTED_GAME,
                found: game,
            }
            .into());
        }

        Ok(Self {
            demo_protocol,
            network_protocol,
            server_name,
            client_name,
            map_name,
            game,
            duration: reader.read_float()?,
            ticks: reader.read_fixed32()?,
            frames: reader.read_fixed32()?,
            signon_length: reader.read_fixed32()?,
        })
    }
}
