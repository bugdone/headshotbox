use std::rc::Rc;

use bitstream_io::BitRead;

use super::{send_tables::Serializer, Object, Property};
use crate::{read::ValveBitReader, BitReader};

pub(super) type Decoder = Rc<dyn Fn(&mut BitReader) -> std::io::Result<Option<Property>>>;

pub(super) struct DecoderCache {
    pub(super) noscale: Decoder,
    pub(super) simtime: Decoder,
    pub(super) bool: Decoder,
    pub(super) coord: Decoder,
    pub(super) unsigned: Decoder,
    pub(super) signed: Decoder,
    pub(super) string: Decoder,
    pub(super) qangle_precise: Decoder,
    pub(super) qangle_coord: Decoder,
    pub(super) uint64: Decoder,
    pub(super) fixed64: Decoder,
    pub(super) normal_vec: Decoder,
}

impl DecoderCache {
    pub(super) fn new() -> Self {
        Self {
            noscale: Rc::new(decode_noscale),
            simtime: Rc::new(decode_simtime),
            coord: Rc::new(decode_coord),
            bool: Rc::new(decode_bool),
            unsigned: Rc::new(decode_unsigned),
            signed: Rc::new(decode_signed),
            string: Rc::new(decode_string),
            qangle_precise: Rc::new(decode_qangle_precise),
            qangle_coord: Rc::new(decode_qangle_coord),
            uint64: Rc::new(decode_uint64),
            fixed64: Rc::new(decode_fixed64),
            normal_vec: Rc::new(decode_normal_vec),
        }
    }

    pub(super) fn decode_float32(
        &self,
        encoder: Option<&str>,
        bit_count: i32,
        low: f32,
        high: f32,
        flags: i32,
    ) -> Decoder {
        match encoder {
            Some("coord") => Rc::clone(&self.coord),
            Some("simtime") => Rc::clone(&self.simtime),
            Some(_) => todo!(),
            None => {
                if bit_count == 0 || bit_count >= 32 {
                    assert!(flags == 0 && low == 0.0 && high == 1.0);
                    return Rc::clone(&self.noscale);
                }
                return decode_quantized(bit_count, low, high, flags);
            }
        }
    }

    pub(super) fn decode_qangle(&self, encoder: Option<&str>, bit_count: i32) -> Decoder {
        match encoder {
            Some("qangle_precise") => Rc::clone(&self.qangle_precise),
            Some("qangle") => {
                if bit_count != 0 {
                    Rc::new(move |reader| {
                        Ok(Some(Property::Vec3([
                            reader.read_angle(bit_count as u32)?,
                            reader.read_angle(bit_count as u32)?,
                            reader.read_angle(bit_count as u32)?,
                        ])))
                    })
                } else {
                    Rc::clone(&self.qangle_coord)
                }
            }
            Some(s) => todo!("{}", s),
            None => todo!(),
        }
    }

    pub(super) fn decode_object(&self, serializer: Rc<Serializer>) -> Decoder {
        Rc::new(move |reader| {
            if reader.read_bit()? {
                let object = Object::new(&serializer);
                Ok(Some(Property::Object(object)))
            } else {
                Ok(None)
            }
        })
    }

    pub(super) fn decode_polymorphic(&self, serializer: Rc<Serializer>) -> Decoder {
        Rc::new(move |reader| {
            if reader.read_bit()? {
                let _polymorphic_index = reader.read_ubitvar()?;
                // TODO this should create an object based on _polymorphic_index
                let object = Object::new(&serializer);
                Ok(Some(Property::Object(object)))
            } else {
                Ok(None)
            }
        })
    }

    pub(super) fn decode_fixed_array(&self, size: u16) -> Decoder {
        Rc::new(move |_| Ok(Some(Property::Array(vec![None; size as usize]))))
    }

    pub(super) fn decode_var_array(&self, init: Option<Property>) -> Decoder {
        Rc::new(move |r| {
            let vec = vec![init.clone(); r.read_varuint32()? as usize];
            Ok(Some(Property::Array(vec)))
        })
    }

    pub(crate) fn decode_vector(
        &self,
        size: u8,
        encoder: Option<&str>,
        bit_count: i32,
        low_value: f32,
        high_value: f32,
        encode_flags: i32,
    ) -> Decoder {
        assert!(
            (bit_count == 0 || bit_count == 32)
                && low_value == 0.0
                && high_value == 1.0
                && encode_flags == 0
        );
        match encoder {
            Some("normal") => {
                assert!(size == 3);
                Rc::clone(&self.normal_vec)
            }
            // TODO: cache
            Some("coord") => match size {
                2 => Rc::new(|r| decode_vector2(r, decode_coord_f32)),
                3 => Rc::new(|r| decode_vector3(r, decode_coord_f32)),
                4 => Rc::new(|r| decode_vector4(r, decode_coord_f32)),
                6 => Rc::new(|r| decode_vector6(r, decode_coord_f32)),
                _ => unreachable!(),
            },
            None => match size {
                2 => Rc::new(|r| decode_vector2(r, decode_noscale_f32)),
                3 => Rc::new(|r| decode_vector3(r, decode_noscale_f32)),
                4 => Rc::new(|r| decode_vector4(r, decode_noscale_f32)),
                6 => Rc::new(|r| decode_vector6(r, decode_noscale_f32)),
                _ => unreachable!(),
            },
            _ => unimplemented!("vector encoder {}", encoder.unwrap()),
        }
    }
}

fn decode_coord_f32(reader: &mut BitReader) -> std::io::Result<f32> {
    reader.read_coord()
}

fn decode_noscale_f32(reader: &mut BitReader) -> std::io::Result<f32> {
    Ok(f32::from_bits(reader.read::<u32>(32)?))
}

fn decode_noscale(reader: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::F32(decode_noscale_f32(reader)?)))
}

fn decode_simtime(reader: &mut BitReader) -> std::io::Result<Option<Property>> {
    let val = reader.read_varuint32()? as f32 * (1.0 / 64.0);
    Ok(Some(Property::F32(val)))
}

fn decode_coord(reader: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::F32(decode_coord_f32(reader)?)))
}

fn decode_bool(r: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::Bool(r.read_bit()?)))
}

fn decode_unsigned(r: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::U32(r.read_varuint32()?)))
}

fn decode_signed(r: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::I32(r.read_signed_varint32()?)))
}

fn decode_uint64(r: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::U64(r.read_varuint64()?)))
}

fn decode_fixed64(r: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::U64(r.read::<u64>(64)?)))
}

fn decode_string(r: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::Str(Box::from(r.read_string()?))))
}

const ROUNDDOWN: i32 = 1 << 0;
const ROUNDUP: i32 = 1 << 1;
const ENCODE_ZERO_EXACTLY: i32 = 1 << 2;
const ENCODE_INTEGERS_EXACTLY: i32 = 1 << 3;

fn decode_quantized(bit_count: i32, low: f32, high: f32, flags: i32) -> Decoder {
    let QuantizedParams {
        bit_count,
        low,
        high,
        flags,
        decode_mul,
    } = quantized_params(bit_count, low, high, flags);

    Rc::new(move |r: &mut BitReader| {
        let val = if flags & ROUNDDOWN != 0 && r.read_bit()? {
            low
        } else if flags & ROUNDUP != 0 && r.read_bit()? {
            high
        } else if flags & ENCODE_ZERO_EXACTLY != 0 && r.read_bit()? {
            0f32
        } else {
            let u = r.read::<u32>(bit_count as u32)?;
            low + (high - low) * (u as f32 * decode_mul)
        };
        Ok(Some(Property::F32(val)))
    })
}

fn validate_flags(low: f32, high: f32, mut flags: i32) -> i32 {
    if (low == 0.0 && flags & ROUNDDOWN != 0) || high == 0.0 && flags & ROUNDUP != 0 {
        flags &= !ENCODE_ZERO_EXACTLY;
    }
    if low == 0.0 && flags & ENCODE_ZERO_EXACTLY != 0 {
        flags |= ROUNDDOWN;
        flags &= !ENCODE_ZERO_EXACTLY;
    }
    if high == 0.0 && flags & ENCODE_ZERO_EXACTLY != 0 {
        flags |= ROUNDUP;
        flags &= !ENCODE_ZERO_EXACTLY;
    }
    let need_to_test_zero = low < 0.0 && high > 0.0;
    if !need_to_test_zero {
        flags &= !ENCODE_ZERO_EXACTLY;
    }
    if flags & ENCODE_INTEGERS_EXACTLY != 0 {
        flags &= !(ROUNDUP | ROUNDDOWN | ENCODE_ZERO_EXACTLY);
    }
    flags
}

#[derive(Debug, PartialEq)]
struct QuantizedParams {
    bit_count: i32,
    low: f32,
    high: f32,
    flags: i32,
    decode_mul: f32,
}

fn quantized_params(
    mut bit_count: i32,
    mut low: f32,
    mut high: f32,
    flags: i32,
) -> QuantizedParams {
    let flags = validate_flags(low, high, flags);
    let mut steps = 1 << bit_count;
    if flags & ROUNDDOWN != 0 {
        high -= (high - low) / steps as f32;
    } else if flags & ROUNDUP != 0 {
        low += (high - low) / steps as f32;
    }
    let low = low;
    if flags & ENCODE_INTEGERS_EXACTLY != 0 {
        let num_ints = (high as i32 - low as i32).max(1);
        let log = num_ints.ilog2() as i32;
        bit_count = bit_count.max(log + 1);
        steps = 1 << bit_count;
        let range = 1 << log;
        high = low + range as f32 - range as f32 / steps as f32;
    }
    let high = high;
    let bit_count = bit_count;
    let decode_mul = 1.0f32 / (steps - 1) as f32;

    let high_low_mul = {
        let range = high - low;
        assert!(low < high);
        let max: u32 = if bit_count == 32 {
            0xFFFFFFFE
        } else {
            (1 << bit_count) - 1
        };
        let too_big = |val| (val * range) as u32 > max || (val * range) > max as f32;
        let mut high_low_mul = max as f32 / range;
        if too_big(high_low_mul) {
            const MULTIPLIERS: [f32; 5] = [0.9999f32, 0.99f32, 0.9f32, 0.8f32, 0.7f32];
            for mul in MULTIPLIERS.iter() {
                high_low_mul = (max as f32 / range) * mul;
                if !too_big(high_low_mul) {
                    break;
                }
            }
        }
        high_low_mul
    };

    let quantize = |val| {
        if val < low {
            low
        } else if val > high {
            high
        } else {
            let u = ((val - low) * high_low_mul) as u32;
            low + (high - low) * ((u as f32) * decode_mul)
        }
    };

    // Remove unnecessary flags
    let flags = {
        let mut flags = flags;
        if flags & ROUNDDOWN != 0 && quantize(low) == low {
            flags &= !ROUNDDOWN;
        }
        if flags & ROUNDUP != 0 && quantize(high) == high {
            flags &= !ROUNDUP;
        }
        if flags & ENCODE_ZERO_EXACTLY != 0 && quantize(0f32) == 0f32 {
            flags &= !ENCODE_ZERO_EXACTLY;
        }
        flags
    };
    QuantizedParams {
        bit_count,
        low,
        high,
        flags,
        decode_mul,
    }
}

fn decode_qangle_precise(r: &mut BitReader) -> std::io::Result<Option<Property>> {
    let mut vec = [0.0; 3];
    let has_x = r.read_bit()?;
    let has_y = r.read_bit()?;
    let has_z = r.read_bit()?;
    if has_x {
        vec[0] = r.read_angle(20)? - 180.0;
    }
    if has_y {
        vec[1] = r.read_angle(20)? - 180.0;
    }
    if has_z {
        vec[2] = r.read_angle(20)? - 180.0;
    }
    Ok(Some(Property::Vec3(vec)))
}

fn decode_qangle_coord(r: &mut BitReader) -> std::io::Result<Option<Property>> {
    let mut vec = [0.0; 3];
    let has_x = r.read_bit()?;
    let has_y = r.read_bit()?;
    let has_z = r.read_bit()?;
    if has_x {
        vec[0] = r.read_coord()?;
    }
    if has_y {
        vec[1] = r.read_coord()?;
    }
    if has_z {
        vec[2] = r.read_coord()?;
    }
    Ok(Some(Property::Vec3(vec)))
}

fn decode_normal_vec(r: &mut BitReader) -> std::io::Result<Option<Property>> {
    let mut vec = [0.0; 3];
    let has_x = r.read_bit()?;
    let has_y = r.read_bit()?;
    if has_x {
        vec[0] = r.read_normal()?;
    }
    if has_y {
        vec[1] = r.read_normal()?;
    }
    let neg_z = r.read_bit()?;
    let prod_sum = vec[0] * vec[0] + vec[1] * vec[1];
    if prod_sum < 1.0 {
        vec[2] = (1.0 - prod_sum).sqrt();
    } else {
        vec[2] = 0.0;
    }
    if neg_z {
        vec[2] = -vec[2];
    }
    Ok(Some(Property::Vec3(vec)))
}

type DecodeF32 = fn(r: &mut BitReader) -> std::io::Result<f32>;

fn decode_vector2(r: &mut BitReader, decoder: DecodeF32) -> std::io::Result<Option<Property>> {
    let vec = [decoder(r)?, decoder(r)?];
    Ok(Some(Property::Vec2(vec)))
}

fn decode_vector3(r: &mut BitReader, decoder: DecodeF32) -> std::io::Result<Option<Property>> {
    let vec = [decoder(r)?, decoder(r)?, decoder(r)?];
    Ok(Some(Property::Vec3(vec)))
}

fn decode_vector4(r: &mut BitReader, decoder: DecodeF32) -> std::io::Result<Option<Property>> {
    let vec = [decoder(r)?, decoder(r)?, decoder(r)?, decoder(r)?];
    Ok(Some(Property::Vec4(vec)))
}

fn decode_vector6(r: &mut BitReader, decoder: DecodeF32) -> std::io::Result<Option<Property>> {
    let vec = [
        decoder(r)?,
        decoder(r)?,
        decoder(r)?,
        decoder(r)?,
        decoder(r)?,
        decoder(r)?,
    ];
    Ok(Some(Property::Vec6(vec)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    #[test]
    fn test_quantized_params() {
        assert_eq!(quantized_params(10, -25f32, 25f32, 2), QuantizedParams {bit_count: 10, low: -24.951172, high: 25f32, flags: 0, decode_mul: 0.0009775171});
        assert_eq!(quantized_params(10, -64f32, 64f32, 4), QuantizedParams {bit_count: 10, low: -64f32, high: 64f32, flags: 4, decode_mul: 0.0009775171});
        assert_eq!(quantized_params(10, 0.100000, 10f32, 0), QuantizedParams {bit_count: 10, low: 0.100000, high: 10f32, flags: 0, decode_mul: 0.0009775171});
        assert_eq!(quantized_params(10, 0f32, 102.3, 2), QuantizedParams {bit_count: 10, low: 0.09990235, high: 102.3, flags: 2, decode_mul: 0.0009775171});
        assert_eq!(quantized_params(10, 0f32, 1024f32, 1), QuantizedParams {bit_count: 10, low: 0f32, high: 1023f32, flags: 0, decode_mul: 0.0009775171});
        assert_eq!(quantized_params(10, 0f32, 256f32, 1), QuantizedParams {bit_count: 10, low: 0f32, high: 255.75, flags: 0, decode_mul: 0.0009775171});
        assert_eq!(quantized_params(11, -1f32, 63f32, 0), QuantizedParams {bit_count: 11, low: -1f32, high: 63f32, flags: 0, decode_mul: 0.0004885198});
        assert_eq!(quantized_params(12, 0f32, 1f32, 1), QuantizedParams {bit_count: 12, low: 0f32, high: 0.99975586, flags: 0, decode_mul: 0.00024420026});
        assert_eq!(quantized_params(12, 0f32, 2048f32, 1), QuantizedParams {bit_count: 12, low: 0f32, high: 2047.5, flags: 0, decode_mul: 0.00024420026});
        assert_eq!(quantized_params(15, 0f32, 1f32, -8), QuantizedParams {bit_count: 15, low: 0f32, high: 0.9999695, flags: -8, decode_mul: 3.051_851e-5});
        assert_eq!(quantized_params(15, 0f32, 1024f32, 1), QuantizedParams {bit_count: 15, low: 0f32, high: 1023.96875, flags: 0, decode_mul: 3.051_851e-5});
        assert_eq!(quantized_params(16, 0f32, 500f32, 1), QuantizedParams {bit_count: 16, low: 0f32, high: 499.992_37, flags: 0, decode_mul: 1.525_902_2e-5});
        assert_eq!(quantized_params(17, -4096f32, 4096f32, 4), QuantizedParams {bit_count: 17, low: -4096f32, high: 4096f32, flags: 4, decode_mul: 7.629_453e-6});
        assert_eq!(quantized_params(18, -4096f32, 4096f32, 4), QuantizedParams {bit_count: 18, low: -4096f32, high: 4096f32, flags: 4, decode_mul: 3.814_712e-6});
        assert_eq!(quantized_params(18, 0f32, 1500f32, 1), QuantizedParams {bit_count: 18, low: 0f32, high: 1_499.994_3, flags: 0, decode_mul: 3.814_712e-6});
        assert_eq!(quantized_params(20, 0f32, 128f32, 4), QuantizedParams {bit_count: 20, low: 0f32, high: 127.99988, flags: 0, decode_mul: 9.536_752e-7});
        assert_eq!(quantized_params(20, 0f32, 256f32, 1), QuantizedParams {bit_count: 20, low: 0f32, high: 255.999_76, flags: 0, decode_mul: 9.536_752e-7});
        assert_eq!(quantized_params(6, 0f32, 64f32, 2), QuantizedParams {bit_count: 6, low: 1f32, high: 64f32, flags: 0, decode_mul: 0.015873017});
        assert_eq!(quantized_params(7, 0f32, 360f32, 1), QuantizedParams {bit_count: 7, low: 0f32, high: 357.1875, flags: 0, decode_mul: 0.007874016});
        assert_eq!(quantized_params(8, -4f32, 12f32, 5), QuantizedParams {bit_count: 8, low: -4f32, high: 11.9375, flags: 0, decode_mul: 0.003921569});
        assert_eq!(quantized_params(8, 0f32, 1f32, -8), QuantizedParams {bit_count: 8, low: 0f32, high: 0.99609375, flags: -8, decode_mul: 0.003921569});
        assert_eq!(quantized_params(8, 0f32, 1f32, 0), QuantizedParams {bit_count: 8, low: 0f32, high: 1f32, flags: 0, decode_mul: 0.003921569});
        assert_eq!(quantized_params(8, 0f32, 1f32, 1), QuantizedParams {bit_count: 8, low: 0f32, high: 0.99609375, flags: 0, decode_mul: 0.003921569});
        assert_eq!(quantized_params(8, 0f32, 100f32, 0), QuantizedParams {bit_count: 8, low: 0f32, high: 100f32, flags: 0, decode_mul: 0.003921569});
        assert_eq!(quantized_params(8, 0f32, 256f32, 1), QuantizedParams {bit_count: 8, low: 0f32, high: 255f32, flags: 0, decode_mul: 0.003921569});
        assert_eq!(quantized_params(8, 0f32, 360f32, 0), QuantizedParams {bit_count: 8, low: 0f32, high: 360f32, flags: 0, decode_mul: 0.003921569});
        assert_eq!(quantized_params(8, 0f32, 4f32, 1), QuantizedParams {bit_count: 8, low: 0f32, high: 3.984375, flags: 0, decode_mul: 0.003921569});
        assert_eq!(quantized_params(8, 0f32, 60f32, 2), QuantizedParams {bit_count: 8, low: 0.234375, high: 60f32, flags: 2, decode_mul: 0.003921569});
        assert_eq!(quantized_params(8, 0f32, 64f32, 1), QuantizedParams {bit_count: 8, low: 0f32, high: 63.75, flags: 0, decode_mul: 0.003921569});

    }
}
