use std::rc::Rc;

use bitstream_io::BitRead;

use super::{property::Object, send_tables::Serializer, Property};
use crate::{read::ValveBitReader, BitReader};

#[derive(Clone)]
pub(super) enum Decoder {
    None,
    Quantized(QuantizedParams),
    NoScale,
    Simtime,
    Bool,
    Coord,
    I32,
    U32,
    U64,
    Fixed64,
    String,
    QAnglePrecise,
    QAngleCoord,
    QAngle(u32),
    VectorNormal,
    VectorCoord(u8),
    VectorNoScale(u8),
    Object(Rc<Serializer>),
    Polymorphic(Rc<Serializer>),
}

impl Decoder {
    pub(super) fn decode(&self, reader: &mut BitReader) -> std::io::Result<Option<Property>> {
        match self {
            Decoder::Quantized(qp) => decode_quantized(reader, qp),
            Decoder::NoScale => decode_noscale(reader),
            Decoder::Simtime => decode_simtime(reader),
            Decoder::Bool => decode_bool(reader),
            Decoder::Coord => decode_coord(reader),
            Decoder::I32 => decode_signed(reader),
            Decoder::U32 => decode_unsigned(reader),
            Decoder::U64 => decode_uint64(reader),
            Decoder::Fixed64 => decode_fixed64(reader),
            Decoder::String => decode_string(reader),
            Decoder::QAnglePrecise => decode_qangle_precise(reader),
            Decoder::QAngleCoord => decode_qangle_coord(reader),
            Decoder::QAngle(bit_count) => decode_qangle3(reader, *bit_count),
            Decoder::VectorNormal => decode_vector_normal(reader),
            Decoder::VectorCoord(size) => decode_vector_coord(reader, *size),
            Decoder::VectorNoScale(size) => decode_vector_noscale(reader, *size),
            Decoder::Object(serializer) => decode_object(reader, serializer),
            Decoder::Polymorphic(serializer) => decode_polymorphic(reader, serializer),
            Decoder::None => unreachable!(),
        }
    }
}

pub(super) fn decode_float32(
    encoder: Option<&str>,
    bit_count: i32,
    low: f32,
    high: f32,
    flags: i32,
) -> Decoder {
    match encoder {
        Some("coord") => Decoder::Coord,
        Some("simtime") => Decoder::Simtime,
        Some(_) => todo!(),
        None => {
            if bit_count == 0 || bit_count >= 32 {
                assert!(flags == 0 && low == 0.0 && high == 1.0);
                Decoder::NoScale
            } else {
                Decoder::Quantized(quantized_params(bit_count, low, high, flags))
            }
        }
    }
}

pub(super) fn decode_qangle(encoder: Option<&str>, bit_count: i32) -> Decoder {
    match encoder {
        Some("qangle_precise") => Decoder::QAnglePrecise,
        Some("qangle") => {
            if bit_count != 0 {
                Decoder::QAngle(bit_count as u32)
            } else {
                Decoder::QAngleCoord
            }
        }
        Some(s) => todo!("{}", s),
        None => todo!(),
    }
}

pub(super) fn decode_object(
    reader: &mut BitReader,
    serializer: &Rc<Serializer>,
) -> std::io::Result<Option<Property>> {
    if reader.read_bit()? {
        let object = Object::new(serializer);
        Ok(Some(Property::Object(object)))
    } else {
        Ok(None)
    }
}

pub(super) fn decode_polymorphic(
    reader: &mut BitReader,
    serializer: &Rc<Serializer>,
) -> std::io::Result<Option<Property>> {
    if reader.read_bit()? {
        let _polymorphic_index = reader.read_ubitvar()?;
        // TODO this should create an object based on _polymorphic_index
        let object = Object::new(serializer);
        Ok(Some(Property::Object(object)))
    } else {
        Ok(None)
    }
}

pub(crate) fn decode_vector(
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
            Decoder::VectorNormal
        }
        Some("coord") => Decoder::VectorCoord(size),
        None => Decoder::VectorNoScale(size),
        _ => unimplemented!("vector encoder {}", encoder.unwrap()),
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

fn decode_bool(reader: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::Bool(reader.read_bit()?)))
}

fn decode_unsigned(reader: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::U32(reader.read_varuint32()?)))
}

fn decode_signed(reader: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::I32(reader.read_signed_varint32()?)))
}

fn decode_uint64(reader: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::U64(reader.read_varuint64()?)))
}

fn decode_fixed64(reader: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::U64(reader.read::<u64>(64)?)))
}

fn decode_string(reader: &mut BitReader) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::Str(Box::from(reader.read_string()?))))
}

const ROUNDDOWN: i32 = 1 << 0;
const ROUNDUP: i32 = 1 << 1;
const ENCODE_ZERO_EXACTLY: i32 = 1 << 2;
const ENCODE_INTEGERS_EXACTLY: i32 = 1 << 3;

fn decode_quantized(r: &mut BitReader, p: &QuantizedParams) -> std::io::Result<Option<Property>> {
    let val = if p.flags & ROUNDDOWN != 0 && r.read_bit()? {
        p.low
    } else if p.flags & ROUNDUP != 0 && r.read_bit()? {
        p.high
    } else if p.flags & ENCODE_ZERO_EXACTLY != 0 && r.read_bit()? {
        0f32
    } else {
        let u = r.read::<u32>(p.bit_count as u32)?;
        p.low + (p.high - p.low) * (u as f32 * p.decode_mul)
    };
    Ok(Some(Property::F32(val)))
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

#[derive(Clone, Debug, PartialEq)]
pub(super) struct QuantizedParams {
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

fn decode_qangle3(reader: &mut BitReader, bit_count: u32) -> std::io::Result<Option<Property>> {
    Ok(Some(Property::Vec3([
        reader.read_angle(bit_count)?,
        reader.read_angle(bit_count)?,
        reader.read_angle(bit_count)?,
    ])))
}

fn decode_qangle_precise(reader: &mut BitReader) -> std::io::Result<Option<Property>> {
    let mut vec = [0.0; 3];
    let has_x = reader.read_bit()?;
    let has_y = reader.read_bit()?;
    let has_z = reader.read_bit()?;
    if has_x {
        vec[0] = reader.read_angle(20)? - 180.0;
    }
    if has_y {
        vec[1] = reader.read_angle(20)? - 180.0;
    }
    if has_z {
        vec[2] = reader.read_angle(20)? - 180.0;
    }
    Ok(Some(Property::Vec3(vec)))
}

fn decode_qangle_coord(reader: &mut BitReader) -> std::io::Result<Option<Property>> {
    let mut vec = [0.0; 3];
    let has_x = reader.read_bit()?;
    let has_y = reader.read_bit()?;
    let has_z = reader.read_bit()?;
    if has_x {
        vec[0] = reader.read_coord()?;
    }
    if has_y {
        vec[1] = reader.read_coord()?;
    }
    if has_z {
        vec[2] = reader.read_coord()?;
    }
    Ok(Some(Property::Vec3(vec)))
}

fn decode_vector_normal(reader: &mut BitReader) -> std::io::Result<Option<Property>> {
    let mut vec = [0.0; 3];
    let has_x = reader.read_bit()?;
    let has_y = reader.read_bit()?;
    if has_x {
        vec[0] = reader.read_normal()?;
    }
    if has_y {
        vec[1] = reader.read_normal()?;
    }
    let neg_z = reader.read_bit()?;
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

fn decode_vector_coord(r: &mut BitReader, size: u8) -> std::io::Result<Option<Property>> {
    match size {
        2 => decode_vector2(r, decode_coord_f32),
        3 => decode_vector3(r, decode_coord_f32),
        4 => decode_vector4(r, decode_coord_f32),
        6 => decode_vector6(r, decode_coord_f32),
        _ => unreachable!(),
    }
}

fn decode_vector_noscale(r: &mut BitReader, size: u8) -> std::io::Result<Option<Property>> {
    match size {
        2 => decode_vector2(r, decode_noscale_f32),
        3 => decode_vector3(r, decode_noscale_f32),
        4 => decode_vector4(r, decode_noscale_f32),
        6 => decode_vector6(r, decode_noscale_f32),
        _ => unreachable!(),
    }
}

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
