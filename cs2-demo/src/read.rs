use bitstream_io::BitRead;
use std::io;

pub(crate) trait ReadExt: io::Read {
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

const COORD_INTEGER_BITS: u32 = 14;
const COORD_FRACTIONAL_BITS: u32 = 5;
const COORD_RESOLUTION: f32 = 1_f32 / (1 << COORD_FRACTIONAL_BITS) as f32;

#[inline]
fn read_coord_fraction<R: io::Read, E: bitstream_io::Endianness>(
    reader: &mut bitstream_io::BitReader<R, E>,
) -> io::Result<f32> {
    Ok(reader.read::<u32>(COORD_FRACTIONAL_BITS)? as f32 * COORD_RESOLUTION)
}

pub(crate) trait ValveBitReader {
    fn read_ubitvar(&mut self) -> io::Result<u32>;
    fn read_varuint32(&mut self) -> io::Result<u32>;
    fn read_signed_varint32(&mut self) -> io::Result<i32>;
    fn read_coord(&mut self) -> io::Result<f32>;
    fn read_normal(&mut self) -> io::Result<f32>;
    fn read_angle(&mut self, bits: u32) -> io::Result<f32>;
    fn read_varuint64(&mut self) -> io::Result<u64>;
    fn read_string(&mut self) -> io::Result<String>;
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

    fn read_varuint32(&mut self) -> io::Result<u32> {
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

    fn read_varuint64(&mut self) -> io::Result<u64> {
        let mut result = 0;
        let mut shift = 0;
        for i in 0..10 {
            let b = self.read::<u8>(8)?;
            result |= ((b & 0x7F) as u64) << shift;
            shift += 7;
            if b & 0x80 == 0 {
                assert!(i < 9 || b <= 1);
                return Ok(result);
            }
        }
        unreachable!()
    }

    fn read_signed_varint32(&mut self) -> io::Result<i32> {
        Ok(zigzag_decode(self.read_varuint32()?))
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

    fn read_normal(&mut self) -> io::Result<f32> {
        let sign = self.read_bit()?;
        let fract = self.read::<u32>(11)? as f32;
        let abs = fract * 1_f32 / ((1 << 11) - 1) as f32;
        Ok(if sign { -abs } else { abs })
    }

    fn read_angle(&mut self, bits: u32) -> io::Result<f32> {
        Ok((self.read::<u32>(bits)? as f32 * 360.0) / (1u64 << bits) as f32)
    }

    fn read_string(&mut self) -> io::Result<String> {
        let mut buf = Vec::new();
        loop {
            let c = self.read::<u8>(8)?;
            if c == 0 {
                break;
            }
            buf.push(c);
        }
        String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
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
        assert_eq!(read.read_varuint32().unwrap(), 1);

        let mut read = BitReader::new(&[0x81, 0x23]);
        assert_eq!(read.read_varuint32().unwrap(), 4481);

        let mut read = BitReader::new(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(read.read_varuint32().unwrap(), 4294967295);
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
