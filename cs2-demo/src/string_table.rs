use protobuf::Message;

use crate::proto::demo::{cdemo_string_tables, CDemoStringTables};
use crate::proto::networkbasetypes::CMsgPlayerInfo;
use crate::{Error, Result};

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
    fn try_new(string_table_item: cdemo_string_tables::Items_t) -> Result<Option<Self>> {
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

#[derive(Debug)]
pub enum StringTable {
    UserInfo(Vec<UserInfo>),
}

pub(crate) fn parse_string_tables(st: CDemoStringTables) -> Result<Vec<StringTable>> {
    let mut tables = Vec::new();
    for table in st.tables {
        if table.table_name() == "userinfo" {
            let mut user_infos = Vec::new();
            for item in table.items {
                if let Some(user_info) = UserInfo::try_new(item)? {
                    user_infos.push(user_info);
                }
            }
            tables.push(StringTable::UserInfo(user_infos));
        }
    }
    Ok(tables)
}
