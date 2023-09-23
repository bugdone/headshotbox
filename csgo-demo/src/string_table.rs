use bitstream_io::BitRead;
use demo_format::BitReader;
use getset::Getters;
use protobuf::CodedInputStream;
use tracing::{instrument, trace};

use crate::read_to_terminator::ReadToTerminator;
use crate::Result;

/// Strings from [`StringTable`].
#[derive(Getters, Debug)]
#[getset(get = "pub")]
pub struct Strings {
    /// Strings name.
    name: String,
    /// Strings data.
    data: Option<Vec<u8>>,
}

impl Strings {
    #[instrument(level = "trace", skip(reader))]
    pub(crate) fn try_new(reader: &mut BitReader) -> Result<Self> {
        // Valve uses 4096 as a limit, but panics if string contains
        // more than 100 chars 🤡
        // https://github.com/ValveSoftware/csgo-demoinfo/blob/049f8dbf49099d3cc544ec5061a7f7252cce7b82/demoinfogo/demofiledump.cpp#L1535
        let name = reader.read_string_to_terminator(100)?;
        if !reader.read_bit()? {
            return Ok(Self { name, data: None });
        }
        let size = reader.read::<u16>(16)?;
        let data = reader.read_to_vec(size as usize)?;

        Ok(Self {
            name,
            data: Some(data),
        })
    }
}

/// String table.
///
/// Description from [Valve's community documentation][Valve Doc]:
///
/// _String tables are simple index tables that contain strings and optional
/// binary data per entry. They can be used to avoid transmitting same strings
/// over and over again and just send their matching string table index
/// instead._
///
/// See [Valve's community documentation on string tables][Valve Doc ST] for more
/// information.
///
/// [Valve Doc]: https://developer.valvesoftware.com/wiki/Networking_Events_&_Messages
/// [Valve Doc ST]: https://developer.valvesoftware.com/wiki/Networking_Events_&_Messages#String_Tables
#[derive(Getters, Debug)]
#[getset(get = "pub")]
pub struct StringTable {
    /// Table's name.
    name: String,
    /// Table's [`Strings`]
    strings: Vec<Strings>,
}

impl StringTable {
    #[instrument(level = "trace", skip(reader))]
    pub(crate) fn try_new(reader: &mut BitReader) -> Result<Self> {
        // Valve uses 256 as a limit
        // https://github.com/ValveSoftware/csgo-demoinfo/blob/049f8dbf49099d3cc544ec5061a7f7252cce7b82/demoinfogo/demofiledump.cpp#L1649
        let name = reader.read_string_to_terminator(256)?;
        let strings_number = reader.read::<u16>(16)?;
        let mut strings: Vec<Strings> = Vec::with_capacity(strings_number as usize);

        for _ in 0..strings_number {
            strings.push(Strings::try_new(reader)?);
        }

        // No idea what it is, but apparently it's useless, so we skip it
        // https://github.com/ValveSoftware/csgo-demoinfo/blob/049f8dbf49099d3cc544ec5061a7f7252cce7b82/demoinfogo/demofiledump.cpp#L1599
        if reader.read_bit()? {
            let strings_number = reader.read::<u16>(16)?;
            for _ in 0..strings_number {
                reader.read_string_to_terminator(4096)?;
                if reader.read_bit()? {
                    let size = reader.read::<u32>(16)?;
                    reader.skip(size)?;
                }
            }
        }

        Ok(Self { name, strings })
    }
}

#[derive(Debug)]
pub(crate) struct StringTables {
    pub tables: Vec<StringTable>,
}

impl StringTables {
    #[instrument(level = "trace", skip(reader))]
    pub(crate) fn try_new(reader: &mut CodedInputStream) -> Result<Self> {
        let size = reader.read_fixed32()?;
        let data = reader.read_raw_bytes(size)?;

        let mut reader = BitReader::new(data.as_slice());
        let tables_number = reader.read(8)?;
        let mut tables: Vec<StringTable> = Vec::with_capacity(tables_number as usize);

        for _ in 0..tables_number {
            let string_table = StringTable::try_new(&mut reader)?;
            trace!(?string_table);
            tables.push(string_table);
        }

        Ok(Self { tables })
    }
}
