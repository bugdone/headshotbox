use super::Descriptor;
use cs2_demo::proto::gameevents::{cmsg_source1legacy_game_event, CMsgSource1LegacyGameEvent};
use serde::{de, forward_to_deserialize_any};
use std::fmt::Display;

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("{0}")]
    Message(String),
    #[error("event {event}.{key} expected type {descriptor_type} but got type {event_type}")]
    DescriptorMismatch {
        event: String,
        key: String,
        descriptor_type: i32,
        event_type: i32,
    },
    #[error("event {event}.{key} has type {event_type} but deserializer requested {rust_type}")]
    TypeMismatch {
        event: String,
        key: String,
        event_type: i32,
        rust_type: &'static str,
    },
}

pub(crate) fn from_cs2_event<'a, T: serde::Deserialize<'a>>(
    event: CMsgSource1LegacyGameEvent,
    descriptor: &'a Descriptor,
) -> Result<T> {
    let mut deserializer = Deserializer {
        event,
        descriptor,
        index: -1,
    };
    T::deserialize(&mut deserializer)
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

struct Deserializer<'de> {
    event: CMsgSource1LegacyGameEvent,
    descriptor: &'de Descriptor,
    index: i32,
}

impl<'a> Deserializer<'a> {
    fn current_ekey(&'a self) -> Result<&'a cmsg_source1legacy_game_event::Key_t> {
        let ekey = &self.event.keys[self.index as usize];
        let desc = &self.descriptor.keys[self.index as usize];
        if ekey.type_() != desc.type_ {
            return Err(Error::DescriptorMismatch {
                event: self.descriptor.name.clone(),
                key: desc.name.clone(),
                descriptor_type: desc.type_,
                event_type: ekey.type_(),
            });
        }
        Ok(ekey)
    }

    fn type_mismatch_error(&'a self, rust_type: &'static str) -> Error {
        let ekey = &self.event.keys[self.index as usize];
        let desc = &self.descriptor.keys[self.index as usize];
        Error::TypeMismatch {
            event: self.descriptor.name.clone(),
            key: desc.name.clone(),
            event_type: ekey.type_(),
            rust_type,
        }
    }
}

impl<'de> de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: de::Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        panic!("GameEvent deserializer internal error")
    }

    forward_to_deserialize_any! { i8 i16 i64 u8 u16 u32 f64 char str bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map
    }

    fn deserialize_bool<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let ekey = self.current_ekey()?;
        let value = match ekey.type_() {
            6 => ekey.val_bool(),
            _ => return Err(self.type_mismatch_error("bool")),
        };
        visitor.visit_bool(value)
    }

    fn deserialize_i32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let ekey = self.current_ekey()?;
        let value = match ekey.type_() {
            3 | 8 => ekey.val_long(),
            4 | 9 => ekey.val_short(),
            5 => ekey.val_byte(),
            _ => return Err(self.type_mismatch_error("i32")),
        };
        visitor.visit_i32(value)
    }

    fn deserialize_u64<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let ekey = self.current_ekey()?;
        let value = match ekey.type_() {
            7 => ekey.val_uint64(),
            _ => return Err(self.type_mismatch_error("u64")),
        };
        visitor.visit_u64(value)
    }

    fn deserialize_f32<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let ekey = self.current_ekey()?;
        let value = match ekey.type_() {
            2 => ekey.val_float(),
            _ => return Err(self.type_mismatch_error("f32")),
        };
        visitor.visit_f32(value)
    }

    fn deserialize_string<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let ekey = self.current_ekey()?;
        let value = match ekey.type_() {
            1 => ekey.val_string(),
            _ => return Err(self.type_mismatch_error("str")),
        };
        visitor.visit_str(value)
    }

    fn deserialize_struct<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_map(self)
    }

    fn deserialize_enum<V: de::Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_enum(Enum { de: self })
    }

    fn deserialize_identifier<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.index == -1 {
            visitor.visit_borrowed_str(self.descriptor.name.as_str())
        } else {
            visitor.visit_borrowed_str(self.descriptor.keys[self.index as usize].name.as_str())
        }
    }

    fn deserialize_ignored_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_bool(false)
    }
}

impl<'de> de::MapAccess<'de> for Deserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        self.index += 1;
        if self.index >= self.descriptor.keys.len() as i32 {
            return Ok(None);
        }
        seed.deserialize(self).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }
}

struct Enum<'a, 'de> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> de::EnumAccess<'de> for Enum<'a, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self.de)?;
        Ok((val, self))
    }
}

impl<'de, 'a> de::VariantAccess<'de> for Enum<'a, 'de> {
    type Error = Error;

    fn unit_variant(self) -> std::result::Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> std::result::Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(
        self,
        _len: usize,
        _visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        panic!("GameEvent deserializer internal error")
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        panic!("GameEvent deserializer internal error")
    }
}
