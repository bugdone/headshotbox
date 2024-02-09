use bitstream_io::BitRead;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use getset::Getters;
use protobuf::CodedInputStream;
use std::ffi::CStr;
use std::io::{BufRead, Cursor};
use tracing::{instrument, trace};

use crate::proto::netmessages::{CSVCMsg_CreateStringTable, CSVCMsg_UpdateStringTable};
use crate::read_to_terminator::ReadToTerminator;
use crate::{num_bits, BitReader, Error, Result};

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

pub(crate) fn parse_string_tables(reader: &mut CodedInputStream) -> Result<Vec<StringTable>> {
    let size = reader.read_fixed32()?;
    let data = reader.read_raw_bytes(size)?;

    let mut reader = BitReader::new(data.as_slice());
    let tables_number = reader.read::<u8>(8)?;
    let mut tables: Vec<StringTable> = Vec::with_capacity(tables_number as usize);

    for _ in 0..tables_number {
        let string_table = StringTable::try_new(&mut reader)?;
        trace!(?string_table);
        tables.push(string_table);
    }

    Ok(tables)
}

#[derive(Debug, Clone)]
struct StringTableDescriptor {
    name: String,
    max_entries: u32,
    user_data_fixed_size: bool,
}

pub struct StringTables {
    string_tables: Vec<StringTableDescriptor>,
}

impl StringTables {
    pub fn new() -> Self {
        Self {
            string_tables: Vec::new(),
        }
    }

    pub fn create_string_table<'a, 's: 'a>(
        &'s mut self,
        table: &'a CSVCMsg_CreateStringTable,
    ) -> StringTableUpdates<'a> {
        self.string_tables.push(StringTableDescriptor {
            name: table.name().to_string(),
            max_entries: table.max_entries() as u32,
            user_data_fixed_size: table.user_data_fixed_size(),
        });
        StringTableUpdates::new(
            self.string_tables.last().unwrap(),
            table.num_entries(),
            table.string_data(),
        )
    }

    pub fn update_string_table<'a, 's: 'a>(
        &'s mut self,
        table: &'a CSVCMsg_UpdateStringTable,
    ) -> Result<StringTableUpdates<'a>> {
        if let Some(table_descriptor) = self.string_tables.get(table.table_id() as usize) {
            Ok(StringTableUpdates::new(
                table_descriptor,
                table.num_changed_entries(),
                table.string_data(),
            ))
        } else {
            Err(Error::StringTable("got bad index for UpdateStringTable"))
        }
    }
}

impl Default for StringTables {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct PlayerInfo {
    pub version: u64,
    pub xuid: u64,
    pub name: String,
    pub user_id: i32,
    pub guid: String,
    pub friends_id: i32,
    pub friends_name: String,
    pub fakeplayer: bool,
    pub is_hltv: bool,
    pub files_downloaded: u8,
    pub entity_id: i32,
}

pub struct StringTableUpdates<'a> {
    table_descriptor: &'a StringTableDescriptor,
    entries: i32,
    reader: BitReader<'a>,
    entry: i32,
    next_entity_id: i32,
    entry_bits: u32,
}

impl<'a> StringTableUpdates<'a> {
    fn new(table_descriptor: &'a StringTableDescriptor, entries: i32, data: &'a [u8]) -> Self {
        let entry_bits = num_bits(table_descriptor.max_entries - 1);
        Self {
            table_descriptor,
            entries,
            reader: BitReader::new(data),
            entry: 0,
            next_entity_id: 0,
            entry_bits,
        }
    }

    pub fn next_player_info(&mut self) -> Result<Option<PlayerInfo>> {
        if self.entry >= self.entries {
            return Ok(None);
        }
        if self.entry == 0 {
            if self.table_descriptor.name != "userinfo" {
                return Ok(None);
            }
            if self.table_descriptor.user_data_fixed_size {
                Err(Error::StringTable("userinfo should not be fixed data"))?;
            }
            if self.reader.read_bit()? {
                Err(Error::StringTable(
                    "cannot decode string table encoded with dictionaries",
                ))?;
            }
        }
        let max_entries = self.table_descriptor.max_entries;
        while self.entry < self.entries {
            let entity_id = if !self.reader.read_bit()? {
                self.reader.read::<u32>(self.entry_bits)? as i32
            } else {
                self.next_entity_id
            };
            self.next_entity_id = entity_id + 1;
            if entity_id >= max_entries as i32 {
                Err(Error::StringTable("update_string_table got a bad index"))?;
            }
            if self.reader.read_bit()? {
                if self.reader.read_bit()? {
                    Err(Error::StringTable("substrings not implemented"))?;
                } else {
                    // I don't know what this is, ignore the string.
                    while self.reader.read::<u8>(8)? != 0 {}
                }
            }
            if self.reader.read_bit()? {
                let num_bytes = self.reader.read::<u32>(14)? as usize;
                let mut buf = vec![0; num_bytes];
                self.reader.read_bytes(buf.as_mut_slice())?;
                let player_info = parse_player_info(&buf, entity_id)?;
                self.entry += 1;
                return Ok(Some(player_info));
            } else {
                self.entry += 1
            }
        }
        Ok(None)
    }
}

fn parse_player_info(buf: &[u8], entity_id: i32) -> Result<PlayerInfo> {
    const PLAYER_NAME_LENGTH: usize = 128;
    const GUID_LENGTH: usize = 33;
    let mut reader = Cursor::new(buf);
    let version = reader.read_u64::<LittleEndian>()?;
    let xuid = reader.read_u64::<BigEndian>()?;
    let name = read_cstring_buffer(&mut reader, PLAYER_NAME_LENGTH)?;
    let user_id = reader.read_i32::<BigEndian>()?;
    let guid = read_cstring_buffer(&mut reader, GUID_LENGTH)?;
    // Skip padding.
    reader.consume(3);
    let friends_id = reader.read_i32::<BigEndian>()?;
    let friends_name = read_cstring_buffer(&mut reader, PLAYER_NAME_LENGTH)?;
    let fakeplayer = reader.read_u8()? != 0;
    let is_hltv = reader.read_u8()? != 0;
    // Skip padding.
    reader.consume(2);
    // Ignore custom_files (4 CRC32 values).
    reader.consume(4 * std::mem::size_of::<u32>());
    let files_downloaded = reader.read_u8()?;
    let player_info = PlayerInfo {
        version,
        xuid,
        name,
        user_id,
        guid,
        friends_id,
        friends_name,
        fakeplayer,
        is_hltv,
        files_downloaded,
        entity_id,
    };
    Ok(player_info)
}

pub fn parse_player_infos(st: Vec<StringTable>) -> Result<Vec<PlayerInfo>> {
    let mut result = Vec::new();
    for st in st.iter().filter(|st| st.name() == "userinfo") {
        for (entity_id, string) in st.strings().iter().enumerate() {
            if let Some(data) = string.data() {
                let player_info = parse_player_info(data, entity_id as i32)?;
                result.push(player_info);
            }
        }
    }
    Ok(result)
}

fn read_cstring_buffer(cursor: &mut Cursor<&[u8]>, size: usize) -> std::io::Result<String> {
    let cstr = CStr::from_bytes_until_nul(&cursor.get_ref()[cursor.position() as usize..])
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    cursor.consume(size);
    Ok(cstr.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::StringTables;
    use crate::Result;
    use protobuf::text_format::parse_from_str;

    #[test]
    fn userinfo_empty_update() -> Result<()> {
        let mut st = StringTables::default();
        let create = parse_from_str(
            r#"name: "userinfo"
               max_entries: 256
               user_data_fixed_size: false
               string_data: "\0""#,
        )
        .unwrap();
        let update = parse_from_str(
            r#"table_id: 0
               num_changed_entries: 1
               string_data: "\020\260""#,
        )
        .unwrap();
        let mut updates = st.create_string_table(&create);
        while (updates.next_player_info()?).is_some() {}
        let mut updates = st.update_string_table(&update)?;
        while (updates.next_player_info()?).is_some() {}
        Ok(())
    }
}
