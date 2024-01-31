use std::collections::VecDeque;

use bitstream_io::BitRead;
use protobuf::Message;

use crate::proto::demo::cdemo_string_tables;
use crate::proto::netmessages::{CSVCMsg_CreateStringTable, CSVCMsg_UpdateStringTable};
use crate::proto::networkbasetypes::CMsgPlayerInfo;
use crate::read::ValveBitReader;
use crate::{BitReader, Error, Result};

#[derive(Debug)]
pub struct PlayerInfo {
    pub name: String,
    pub xuid: u64,
    pub user_id: i32,
    pub fakeplayer: bool,
    pub is_hltv: bool,
}

#[derive(Debug)]
pub struct UserInfo {
    pub index: u16,
    pub info: PlayerInfo,
}

impl UserInfo {
    pub(super) fn try_new(
        string_table_item: &cdemo_string_tables::Items_t,
    ) -> Result<Option<Self>> {
        let Some(ref data) = string_table_item.data else {
            return Ok(None);
        };
        let index = string_table_item
            .str()
            .parse()
            .or(Err(Error::InvalidPlayerIndex))?;
        let msg = CMsgPlayerInfo::parse_from_bytes(data)?;
        if msg.ishltv() || msg.fakeplayer() {
            return Ok(None);
        }
        Ok(Some(UserInfo {
            index,
            info: PlayerInfo {
                name: msg.name().into(),
                xuid: msg.xuid(),
                user_id: msg.userid(),
                fakeplayer: msg.fakeplayer(),
                is_hltv: msg.ishltv(),
            },
        }))
    }
}

pub(crate) fn parse_userinfo(table: &cdemo_string_tables::Table_t) -> Result<Vec<UserInfo>> {
    let mut user_infos = Vec::new();
    for item in &table.items {
        if let Some(user_info) = UserInfo::try_new(item)? {
            user_infos.push(user_info);
        }
    }
    Ok(user_infos)
}

pub(crate) struct StringTableInfo {
    pub(crate) name: String,
    user_data_size: i32,
    user_data_fixed_size: bool,
    flags: i32,
    using_varint_bitcounts: bool,
}

pub(crate) const INSTANCEBASELINE: &str = "instancebaseline";
pub(crate) const USERINFO: &str = "userinfo";
pub(crate) type StringTableData = Vec<(String, Vec<u8>)>;

pub(crate) fn parse_create_string_table(
    mut msg: CSVCMsg_CreateStringTable,
) -> Result<(StringTableInfo, StringTableData)> {
    let bytes = if msg.data_compressed() {
        snap::raw::Decoder::new().decompress_vec(msg.string_data())?
    } else {
        msg.take_string_data()
    };
    let info = StringTableInfo {
        name: msg.take_name(),
        user_data_size: msg.user_data_size(),
        user_data_fixed_size: msg.user_data_fixed_size(),
        flags: msg.flags(),
        using_varint_bitcounts: msg.using_varint_bitcounts(),
    };
    let data = parse_string_table(&info, bytes, msg.num_entries())?;
    Ok((info, data))
}

pub(crate) fn parse_update_string_table(
    mut msg: CSVCMsg_UpdateStringTable,
    info: &StringTableInfo,
) -> Result<StringTableData> {
    parse_string_table(info, msg.take_string_data(), msg.num_changed_entries())
}

fn parse_string_table(
    info: &StringTableInfo,
    bytes: Vec<u8>,
    num_entries: i32,
) -> Result<StringTableData> {
    let mut reader = BitReader::new(&bytes);
    let mut _index = 0;
    let mut history: VecDeque<String> = VecDeque::new();
    let mut items = StringTableData::new();
    for _ in 0..num_entries {
        if !reader.read_bit()? {
            _index += reader.read_varuint32()? as i32
        }
        if reader.read_bit()? {
            let key = if !reader.read_bit()? {
                reader.read_string()?
            } else {
                let position = reader.read::<u32>(5)? as usize;
                let length = reader.read::<u32>(5)? as usize;
                if position >= history.len() {
                    reader.read_string()?
                } else {
                    let s = history[position].as_str();
                    s[0..length.min(s.len())].to_owned() + &reader.read_string()?
                }
            };

            if history.len() >= 32 {
                history.pop_front();
            }
            history.push_back(key.clone());

            let value = if reader.read_bit()? {
                let mut is_compressed = false;
                let bits = if info.user_data_fixed_size {
                    info.user_data_size as u32
                } else {
                    if (info.flags & 0x1) != 0 {
                        is_compressed = reader.read_bit()?;
                    }
                    if info.using_varint_bitcounts {
                        reader.read_ubitvar()? * 8
                    } else {
                        reader.read::<u32>(17)? * 8
                    }
                };
                let value = reader.read_to_vec((bits / 8) as usize)?;
                if is_compressed {
                    snap::raw::Decoder::new().decompress_vec(&value)?
                } else {
                    value
                }
            } else {
                Vec::new()
            };
            items.push((key, value));
        }
        _index += 1;
    }
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata;

    #[test]
    fn test_parse_userinfo() {
        for table in testdata::string_tables().tables {
            if table.table_name() == "userinfo" {
                parse_userinfo(&table).unwrap();
            }
        }
    }

    #[test]
    fn test_parse_create_string_table() -> Result<()> {
        let msg = testdata::create_string_table();
        let (_, data) = parse_create_string_table(msg)?;
        assert_eq!(data.len(), 69);
        Ok(())
    }

    #[test]
    fn test_parse_update_string_table() -> Result<()> {
        let (info, _) = parse_create_string_table(testdata::create_string_table())?;
        let data = parse_update_string_table(testdata::update_string_table(), &info)?;
        assert_eq!(data.len(), 2);
        Ok(())
    }
}
