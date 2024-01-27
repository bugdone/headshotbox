use std::rc::Rc;

use bitstream_io::BitRead;
use demo_format::{read::ValveBitReader, BitReader};

use super::Property;

pub(super) type Decoder = Rc<dyn Fn(&mut BitReader) -> std::io::Result<Property>>;

pub(super) struct DecoderCache {
    noscale: Decoder,
    simtime: Decoder,
    bool: Decoder,
    todo: Decoder,
    unsigned: Decoder,
    signed: Decoder,
    string: Decoder,
}

impl DecoderCache {
    pub(super) fn new() -> Self {
        Self {
            noscale: Rc::new(decode_noscale),
            simtime: Rc::new(decode_simtime),
            todo: Rc::new(decode_todo),
            bool: Rc::new(decode_bool),
            unsigned: Rc::new(decode_unsigned),
            signed: Rc::new(decode_signed),
            string: Rc::new(decode_string),
        }
    }

    pub(super) fn make_decoder(
        &mut self,
        var_type: &str,
        encoder: Option<&str>,
        bit_count: i32,
        low_value: f32,
        high_value: f32,
        encode_flags: i32,
    ) -> Decoder {
        match Type::parse(var_type) {
            Type("float32", None, false) => {
                self.decode_float32(encoder, bit_count, low_value, high_value, encode_flags)
            }
            Type("bool", None, false) => Rc::clone(&self.bool),
            Type("char", None, true) => Rc::clone(&self.string),
            Type("int32", None, false) => Rc::clone(&self.signed),
            Type("uint16", None, false) => Rc::clone(&self.unsigned),
            Type("PlayerConnectedState", None, false) => Rc::clone(&self.unsigned),
            Type("CHandle", Some(_), false) => Rc::clone(&self.unsigned),
            _ => Rc::clone(&self.todo),
        }
    }

    fn decode_float32(
        &self,
        encoder: Option<&str>,
        bit_count: i32,
        _low_value: f32,
        _high_value: f32,
        _encode_flags: i32,
    ) -> Decoder {
        match encoder {
            Some("coord") => Rc::clone(&self.todo),
            Some("simtime") => Rc::clone(&self.simtime),
            Some(_) => todo!(),
            None => {
                if bit_count == 0 || bit_count >= 32 {
                    return Rc::clone(&self.noscale);
                }
                Rc::clone(&self.todo)
            }
        }
    }
}

struct Type<'a>(&'a str, Option<&'a str>, bool);

impl<'a> Type<'a> {
    fn parse(s: &'a str) -> Self {
        let array = s.split_once('[');
        let is_array = array.is_some();
        let s = match array {
            Some((s, _)) => s,
            None => s,
        };
        let param_start = s.find('<');
        let (base, param) = match param_start {
            Some(open) => {
                let close = s.rfind('>').unwrap();
                (&s[..open], Some(&s[open + 1..close]))
            }
            None => (s, None),
        };
        Type(base, param, is_array)
    }
}

fn decode_noscale(reader: &mut BitReader) -> std::io::Result<Property> {
    let val = f32::from_bits(reader.read::<u32>(32)?);
    Ok(Property::F32(val))
}

fn decode_simtime(reader: &mut BitReader) -> std::io::Result<Property> {
    let val = reader.read_varint32()? as f32 * (1.0 / 64.0);
    Ok(Property::F32(val))
}

fn decode_bool(r: &mut BitReader) -> std::io::Result<Property> {
    Ok(Property::Bool(r.read_bit()?))
}

fn decode_unsigned(r: &mut BitReader) -> std::io::Result<Property> {
    Ok(Property::U32(r.read_varint32()?))
}

fn decode_signed(r: &mut BitReader) -> std::io::Result<Property> {
    Ok(Property::I32(r.read_signed_varint32()?))
}

fn decode_string(r: &mut BitReader) -> std::io::Result<Property> {
    let mut buf = Vec::new();
    loop {
        let c = r.read::<u8>(8)?;
        if c == 0 {
            break;
        }
        buf.push(c);
    }
    match std::str::from_utf8(&buf) {
        Ok(s) => Ok(Property::Str(Box::from(s))),
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    }
}

fn decode_todo(_: &mut BitReader) -> std::io::Result<Property> {
    todo!()
}
