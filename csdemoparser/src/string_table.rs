use std::ffi::CStr;
use std::io::{BufRead, Cursor};

use anyhow::bail;
use bitstream_io::BitRead;
use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use csgo_demo::proto::netmessages::{CSVCMsg_CreateStringTable, CSVCMsg_UpdateStringTable};
use demo_format::BitReader;

use crate::num_bits;

#[derive(Debug, Clone)]
struct StringTableDescriptor {
    name: String,
    max_entries: u32,
    user_data_fixed_size: bool,
}

pub(crate) struct StringTables {
    string_tables: Vec<StringTableDescriptor>,
}

impl StringTables {
    pub(crate) fn new() -> Self {
        Self {
            string_tables: Vec::new(),
        }
    }

    pub(crate) fn create_string_table<'a, 's: 'a>(
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

    pub(crate) fn update_string_table<'a, 's: 'a>(
        &'s mut self,
        table: &'a CSVCMsg_UpdateStringTable,
    ) -> anyhow::Result<StringTableUpdates<'a>> {
        if let Some(table_descriptor) = self.string_tables.get(table.table_id() as usize) {
            Ok(StringTableUpdates::new(
                table_descriptor,
                table.num_changed_entries(),
                table.string_data(),
            ))
        } else {
            bail!("got bad index for UpdateStringTable")
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Default)]
pub(crate) struct PlayerInfo {
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

pub(crate) struct StringTableUpdates<'a> {
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

    pub(crate) fn next(&mut self) -> anyhow::Result<Option<PlayerInfo>> {
        if self.entry >= self.entries {
            return Ok(None);
        }
        if self.entry == 0 {
            if self.table_descriptor.name != "userinfo" {
                return Ok(None);
            }
            if self.table_descriptor.user_data_fixed_size {
                bail!("userinfo should not be fixed data");
            }
            if self.reader.read_bit()? {
                bail!("cannot decode string table encoded with dictionaries");
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
                bail!("update_string_table got a bad index");
            }
            if self.reader.read_bit()? {
                if self.reader.read_bit()? {
                    bail!("substrings not implemented");
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

pub(crate) fn parse_player_info(buf: &[u8], entity_id: i32) -> anyhow::Result<PlayerInfo> {
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

fn read_cstring_buffer(cursor: &mut Cursor<&[u8]>, size: usize) -> anyhow::Result<String> {
    let cstr = CStr::from_bytes_until_nul(&cursor.get_ref()[cursor.position() as usize..])?;
    cursor.consume(size);
    Ok(cstr.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::StringTables;
    use protobuf::text_format::parse_from_str;

    #[test]
    fn userinfo_empty_update() -> anyhow::Result<()> {
        let mut st = StringTables::new();
        let create = parse_from_str(
            r#"name: "userinfo"
               max_entries: 256
               user_data_fixed_size: false
               string_data: "\0""#,
        )?;
        let update = parse_from_str(
            r#"table_id: 0
               num_changed_entries: 1
               string_data: "\020\260""#,
        )?;
        let mut updates = st.create_string_table(&create);
        while (updates.next()?).is_some() {}
        let mut updates = st.update_string_table(&update)?;
        while (updates.next()?).is_some() {}
        Ok(())
    }
}
