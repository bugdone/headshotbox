use crate::proto::cstrike15_usermessages::*;
use crate::proto::netmessages::*;
use crate::Result;
use paste::paste;
use protobuf::Message as ProtoMessage;
use protobuf::{CodedInputStream, Enum};
use std::fmt;

/// Generates an enum with a variant for each supported Packet message type.
///
/// $enum is a proto enum listing the Packet message identifiers for a category of messages
/// $enum_prefix is the prefix for all the items in $enum
/// $msg_prefix is the prefix for proto message type names
/// $name is the proto message type name without $msg_prefix
macro_rules! create_message_impl {
    (
        $(
            ($enum:ident, $enum_prefix:ident, $msg_prefix:ident) => [ $($name:ident),* ],
        ),*
        usr = [ $($usr_msg:ident),* ]
    ) => {
        pub enum Message {
            Unknown(i32),
            $($(
                $name(paste!{[<$msg_prefix $name>]})
            ),*),*
            ,
            $(
                $usr_msg(paste!{[<CUSRMsg_ $usr_msg>]})
            ),*
        }

        impl Message {
            pub(crate) fn try_new(reader: &mut CodedInputStream) -> Result<Message> {
                let msg_type = reader.read_int32()?;

                // Handle "User message" before anything else
                //
                // User Messages are particular, because they are SVC messages
                // that contains a User message. This piece of code does the
                // "conversion".
                if msg_type == SVC_Messages::svc_UserMessage as i32 {
                    let svc_message: CSVCMsg_UserMessage = reader.read_message()?;
                    let data = svc_message.msg_data();

                    if let Some(message_type) = USR_Messages::from_i32(svc_message.msg_type()) {
                        paste! {
                            return Ok(match message_type {
                                $(
                                    USR_Messages::[<usr_ $usr_msg>] => Message::$usr_msg([<CUSRMsg_ $usr_msg>]::parse_from_bytes(&data)?),
                                )*
                                // Fallback for unhandled User messages
                                _ => Message::UserMessage(svc_message.clone()),
                            });
                        }
                    }
                }

                paste!{
                    $($(
                        const [<$name:upper>]: i32 = $enum::[<$enum_prefix $name>] as i32;
                    )*)*
                    match msg_type {
                        $($(
                            [<$name:upper>] => Ok(Message::$name(reader.read_message()?)),
                        )*)*
                        _ => {
                            let size = reader.read_raw_varint64()?;
                            reader.skip_raw_bytes(size as u32)?;
                            Ok(Message::Unknown(msg_type))
                        }
                    }
                }
            }
        }

        impl fmt::Debug for Message {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    Message::Unknown(t) => write!(f, "Unknown({})", t),
                    $($(Message::$name(m) => write!(f, "{}({})", stringify!($name), m),)*)*
                    $(Message::$usr_msg(m) => write!(f, "{}({})", stringify!($usr_msg), m),)*
                }
            }
        }
    };
}

create_message_impl! {
    (SVC_Messages, svc_, CSVCMsg_) => [
        SendTable,
        ServerInfo,
        CreateStringTable,
        UpdateStringTable,
        UserMessage,
        GameEventList,
        GameEvent,
        PacketEntities
    ],
    usr = [
        ServerRankUpdate
    ]
}
