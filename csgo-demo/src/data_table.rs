use byteorder::{ReadBytesExt, LE};
use getset::Getters;
use protobuf::CodedInputStream;
use tracing::{instrument, trace};

use crate::error::{DataTablesParsingError, Result};
use crate::message::Message;
use crate::proto::netmessages::{csvcmsg_class_info, CSVCMsg_SendTable};
use crate::read_to_terminator::ReadToTerminator;

#[derive(Getters, Debug)]
#[getset(get = "pub")]
pub struct DataTables {
    send_tables: Vec<CSVCMsg_SendTable>,
    server_classes: Vec<csvcmsg_class_info::Class_t>,
}

impl DataTables {
    #[instrument(level = "trace", skip(reader))]
    pub(crate) fn try_new(reader: &mut CodedInputStream) -> Result<Self> {
        // Read the size of the packet. We don't care about the value so we
        // could just skip it but w/e.
        reader.read_fixed32()?;

        // We have no way of knowing how many send tables there are, so we need
        // to read messages until we found one which contains `is_end == true`
        trace!("reading send tables");
        let mut send_tables: Vec<CSVCMsg_SendTable> = Vec::new();
        loop {
            let message = Message::try_new(reader)?;
            trace!(?message);
            match message {
                Message::SendTable(send_table) => {
                    let is_end = send_table.is_end();
                    send_tables.push(send_table);
                    if is_end {
                        break;
                    }
                }
                _ => {
                    return Err(DataTablesParsingError::NotASendTable.into());
                }
            }
        }

        trace!("reading server classes");
        let class_count = reader.read_u16::<LE>()? as usize;
        let mut server_classes: Vec<csvcmsg_class_info::Class_t> = Vec::with_capacity(class_count);

        for _ in 0..class_count {
            let mut server_class = csvcmsg_class_info::Class_t::new();

            let class_id = reader.read_u16::<LE>()?;
            if class_id as usize >= class_count {
                return Err(DataTablesParsingError::InvalidServerClassIndex {
                    expected_max: class_count,
                    found: class_id.into(),
                }
                .into());
            }

            server_class.set_class_id(class_id.into());
            server_class.set_class_name(reader.read_string_to_terminator(256)?);
            server_class.set_data_table_name(reader.read_string_to_terminator(256)?);

            trace!(?server_class);
            server_classes.push(server_class);
        }

        Ok(Self {
            send_tables,
            server_classes,
        })
    }
}
