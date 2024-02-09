use bitstream_io::BitRead;
use std::io;

pub trait ReadExt: io::Read {
    /// Read `size` bytes and convert them to a [`String`].
    /// Every trailing null terminator will be removed.
    fn read_string_limited(&mut self, size: usize) -> io::Result<String>;
}

impl<R: io::Read> ReadExt for R {
    fn read_string_limited(&mut self, size: usize) -> io::Result<String> {
        let mut buf: Vec<u8> = vec![0; size];
        self.read_exact(buf.as_mut_slice())?;

        let s = String::from_utf8_lossy(&buf).into_owned();
        Ok(s.trim_end_matches('\0').to_string())
    }
}

pub enum CoordType {
    None,
    LowPrecision,
    Integral,
}

const COORD_INTEGER_BITS: u32 = 14;
const COORD_INTEGER_BITS_MP: u32 = 11;
const COORD_FRACTIONAL_BITS: u32 = 5;
const COORD_FRACTIONAL_BITS_LOWPRECISION: u32 = 3;
const COORD_RESOLUTION: f32 = 1_f32 / (1 << COORD_FRACTIONAL_BITS) as f32;
const COORD_RESOLUTION_LOWPRECISION: f32 = 1_f32 / (1 << COORD_FRACTIONAL_BITS_LOWPRECISION) as f32;

#[inline]
fn read_coord_fraction<R: io::Read, E: bitstream_io::Endianness>(
    reader: &mut bitstream_io::BitReader<R, E>,
) -> io::Result<f32> {
    Ok(reader.read::<u32>(COORD_FRACTIONAL_BITS)? as f32 * COORD_RESOLUTION)
}

#[inline]
fn read_coord_fraction_low_precision<R: io::Read, E: bitstream_io::Endianness>(
    reader: &mut bitstream_io::BitReader<R, E>,
) -> io::Result<f32> {
    Ok(
        reader.read::<u32>(COORD_FRACTIONAL_BITS_LOWPRECISION)? as f32
            * COORD_RESOLUTION_LOWPRECISION,
    )
}

pub trait ValveBitReader {
    fn read_ubitvar(&mut self) -> io::Result<u32>;
    fn read_varint32(&mut self) -> io::Result<u32>;
    fn read_signed_varint32(&mut self) -> io::Result<i32>;

    fn read_coord(&mut self) -> io::Result<f32>;
    fn skip_coord(&mut self) -> io::Result<()>;

    fn read_coord_mp(&mut self, coord_type: CoordType) -> io::Result<f32>;
    fn skip_coord_mp(&mut self, coord_type: CoordType) -> io::Result<()>;

    fn read_cell_coord(&mut self, int_bits: u32, coord_type: CoordType) -> io::Result<f32>;
    fn skip_cell_coord(&mut self, int_bits: u32, coord_type: CoordType) -> io::Result<()>;

    fn read_normal(&mut self) -> io::Result<f32>;
    fn skip_normal(&mut self) -> io::Result<()>;
}

impl<R: io::Read> ValveBitReader for bitstream_io::BitReader<R, bitstream_io::LittleEndian> {
    /// Read a 32-bit value using the UBitVar Valve format.
    fn read_ubitvar(&mut self) -> io::Result<u32> {
        let tmp = self.read::<u32>(6)?;
        let last4 = tmp & 15;
        match tmp & (16 | 32) {
            16 => {
                let ret = (self.read::<u32>(4)? << 4) | last4;
                debug_assert!(ret >= 16);
                Ok(ret)
            }
            32 => {
                let ret = (self.read::<u32>(8)? << 4) | last4;
                debug_assert!(ret >= 256);
                Ok(ret)
            }
            48 => {
                let ret = (self.read::<u32>(32 - 4)? << 4) | last4;
                debug_assert!(ret >= 4096);
                Ok(ret)
            }
            _ => Ok(last4),
        }
    }

    fn read_varint32(&mut self) -> io::Result<u32> {
        let mut result = 0;
        for byte in 0..5 {
            let b = self.read::<u8>(8)?;
            result |= ((b & 0x7F) as u32) << (byte * 7);
            if b & 0x80 == 0 {
                break;
            }
        }
        Ok(result)
    }

    fn read_signed_varint32(&mut self) -> io::Result<i32> {
        Ok(zigzag_decode(self.read_varint32()?))
    }

    fn read_coord(&mut self) -> io::Result<f32> {
        let int = self.read_bit()?;
        let fract = self.read_bit()?;
        if !int && !fract {
            return Ok(0_f32);
        }
        let sign = self.read_bit()?;
        let intval = if int {
            self.read::<u32>(COORD_INTEGER_BITS)? + 1
        } else {
            0
        };
        let fractval = if fract {
            read_coord_fraction(self)?
        } else {
            0_f32
        };
        let abs = intval as f32 + fractval;
        Ok(if sign { -abs } else { abs })
    }

    fn skip_coord(&mut self) -> io::Result<()> {
        let int_bits = self.read_bit()? as u32 * COORD_INTEGER_BITS;
        let fract_bits = self.read_bit()? as u32 * COORD_FRACTIONAL_BITS;
        let bits = int_bits + fract_bits + if int_bits + fract_bits > 0 { 1 } else { 0 };
        self.skip(bits)
    }

    fn read_coord_mp(&mut self, coord_type: CoordType) -> io::Result<f32> {
        let int_bits = if self.read_bit()? {
            COORD_INTEGER_BITS_MP
        } else {
            COORD_INTEGER_BITS
        };
        let has_int = self.read_bit()?;
        let negative = match coord_type {
            CoordType::Integral if !has_int => false,
            _ => self.read_bit()?,
        };
        let int = if has_int {
            self.read::<u32>(int_bits)? + 1
        } else {
            0
        };
        let fract = match coord_type {
            CoordType::None => read_coord_fraction(self)?,
            CoordType::LowPrecision => read_coord_fraction_low_precision(self)?,
            CoordType::Integral => 0_f32,
        };
        let abs = int as f32 + fract;
        Ok(if negative { -abs } else { abs })
    }

    fn skip_coord_mp(&mut self, coord_type: CoordType) -> io::Result<()> {
        let mut int_bits = if self.read_bit()? {
            COORD_INTEGER_BITS_MP
        } else {
            COORD_INTEGER_BITS
        };
        let has_int = self.read_bit()?;
        if !has_int {
            int_bits = 0;
        }
        let bits = match coord_type {
            CoordType::None => int_bits + 1 + COORD_FRACTIONAL_BITS,
            CoordType::LowPrecision => int_bits + 1 + COORD_FRACTIONAL_BITS_LOWPRECISION,
            CoordType::Integral => int_bits + has_int as u32,
        };
        self.skip(bits)
    }

    fn read_cell_coord(&mut self, int_bits: u32, coord_type: CoordType) -> io::Result<f32> {
        let int = self.read::<u32>(int_bits)? as f32;
        let val = match coord_type {
            CoordType::None => int + read_coord_fraction(self)?,
            CoordType::LowPrecision => int + read_coord_fraction_low_precision(self)?,
            CoordType::Integral => int,
        };
        Ok(val)
    }

    fn skip_cell_coord(&mut self, int_bits: u32, coord_type: CoordType) -> io::Result<()> {
        match coord_type {
            CoordType::None => self.skip(int_bits + COORD_FRACTIONAL_BITS),
            CoordType::LowPrecision => self.skip(int_bits + COORD_FRACTIONAL_BITS_LOWPRECISION),
            CoordType::Integral => self.skip(int_bits),
        }
    }

    fn read_normal(&mut self) -> io::Result<f32> {
        let sign = self.read_bit()?;
        let fract = self.read::<u32>(11)? as f32;
        let abs = fract * 1_f32 / ((1 << 11) - 1) as f32;
        Ok(if sign { -abs } else { abs })
    }

    fn skip_normal(&mut self) -> io::Result<()> {
        self.skip(12)
    }
}

/// The ZigZag transform is used to minimize the number of bytes needed by the unsigned varint
/// encoding.
///
/// Otherwise, small negative numbers like -1 reinterpreted as u32 would be a very large number
/// and use all 5 bytes in the varint encoding.
fn zigzag_decode(n: u32) -> i32 {
    ((n >> 1) as i32) ^ -((n & 1) as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BitReader;

    #[test]
    fn varint() {
        let mut read = BitReader::new(&[0x01]);
        assert_eq!(read.read_varint32().unwrap(), 1);

        let mut read = BitReader::new(&[0x81, 0x23]);
        assert_eq!(read.read_varint32().unwrap(), 4481);

        let mut read = BitReader::new(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(read.read_varint32().unwrap(), 4294967295);
    }

    #[test]
    fn zigzag() {
        let cases: [(u32, i32); 7] = [
            (0, 0),
            (1, -1),
            (2, 1),
            (3, -2),
            (4, 2),
            (4294967294, 2147483647),
            (4294967295, -2147483648),
        ];
        for (encoded, decoded) in cases {
            assert_eq!(
                decoded,
                zigzag_decode(encoded),
                "zigzag_decode({encoded}) should be {decoded}"
            );
        }
    }
}
