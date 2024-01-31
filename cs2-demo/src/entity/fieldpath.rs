#![allow(non_snake_case)]
use bitstream_io::huffman::{compile_read_tree, ReadHuffmanTree};
use bitstream_io::{BitRead, HuffmanRead, LittleEndian};
use std::sync::OnceLock;
use smallvec::SmallVec;

use crate::{read::ValveBitReader, BitReader};

#[derive(Debug, Clone)]
pub(super) struct FieldPath {
    /// Indices into the serializer.
    pub(super) data: SmallVec<[i32; 6]>,
    /// Indicates if this is the last field path for an entity.
    pub(super) finished: bool,
}

impl FieldPath {
    pub(super) fn new() -> Self {
        let mut data = SmallVec::new();
        data.push(-1);
        let finished = false;
        Self { data, finished }
    }

    pub(super) fn read(&mut self, reader: &mut BitReader) -> std::io::Result<()> {
        type FieldPathOp = fn(&mut FieldPath, &mut BitReader) -> std::io::Result<()>;
        static MEM: OnceLock<Box<[ReadHuffmanTree<LittleEndian, FieldPathOp>]>> = OnceLock::new();
        // Each field path is read using a operation. Field path operations are
        // organized in fixed Huffman tree.
        #[rustfmt::skip]
        let huffman = MEM.get_or_init(|| {
            compile_read_tree::<LittleEndian, FieldPathOp>(vec![
                (PlusOne, vec![0]),
                (FieldPathEncodeFinish, vec![1, 0]),
                (PlusTwo, vec![1, 1, 1, 0]),
                (PushOneLeftDeltaNRightNonZeroPack6Bits, vec![1, 1, 1, 1]),
                (PushOneLeftDeltaOneRightNonZero, vec![1, 1, 0, 0, 0]),
                (PlusN, vec![1, 1, 0, 1, 0]),
                (PlusThree, vec![1, 1, 0, 0, 1, 0]),
                (PopAllButOnePlusOne, vec![1, 1, 0, 0, 1, 1]),
                (PushOneLeftDeltaNRightNonZero, vec![1, 1, 0, 1, 1, 0, 0, 1]),
                (PushOneLeftDeltaOneRightZero, vec![1, 1, 0, 1, 1, 0, 1, 0]),
                (PushOneLeftDeltaNRightZero, vec![1, 1, 0, 1, 1, 1, 0, 0]),
                (PopAllButOnePlusNPack6Bits, vec![1, 1, 0, 1, 1, 1, 1, 0]),
                (PlusFour, vec![1, 1, 0, 1, 1, 1, 1, 1]),
                (PopAllButOnePlusN, vec![1, 1, 0, 1, 1, 0, 0, 0, 0]),
                (PushOneLeftDeltaNRightNonZeroPack8Bits, vec![1, 1, 0, 1, 1, 0, 1, 1, 0]),
                (NonTopoPenultimatePlusOne, vec![1, 1, 0, 1, 1, 0, 1, 1, 1]),
                (PopAllButOnePlusNPack3Bits, vec![1, 1, 0, 1, 1, 1, 0, 1, 0]),
                (PushNAndNonTopological, vec![1, 1, 0, 1, 1, 1, 0, 1, 1]),
                (NonTopoComplexPack4Bits, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 0]),
                (NonTopoComplex, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 1]),
                (PushOneLeftDeltaZeroRightZero, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 1]),
                (PopOnePlusOne, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1]),
                (PushOneLeftDeltaZeroRightNonZero, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 0, 1]),
                (PopNAndNonTopographical, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0]),
                (PopNPlusN, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 1]),
                (PushN, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 0, 0]),
                (PushThreePack5LeftDeltaN, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 0, 1]),
                (PopNPlusOne, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 1, 0]),
                (PopOnePlusN, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 1, 1]),
                (PushTwoLeftDeltaZero, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 0]),
                (PushThreeLeftDeltaZero, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 1, 0]),
                (PushTwoPack5LeftDeltaZero, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 0, 0, 1, 1]),
                (PushTwoLeftDeltaN, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 0]),
                (PushThreePack5LeftDeltaOne, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1]),
                (PushThreeLeftDeltaN, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 0]),
                (PushTwoPack5LeftDeltaN, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 1, 0, 1, 1]),
                (PushTwoLeftDeltaOne, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 0, 0]),
                (PushThreePack5LeftDeltaZero, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 0, 1]),
                (PushThreeLeftDeltaOne, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 0]),
                (PushTwoPack5LeftDeltaOne, vec![1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 1]),
            ]).unwrap()
        });
        let op = reader.read_huffman(huffman)?;
        op(self, reader)
    }
}

impl std::fmt::Display for FieldPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data[0])?;
        for e in &self.data[1..] {
            write!(f, "/{}", e)?;
        }
        Ok(())
    }
}

pub(super) trait FieldPathBitReader {
    fn read_fp_bitvar(&mut self) -> std::io::Result<i32>;
}

impl FieldPathBitReader for BitReader<'_> {
    fn read_fp_bitvar(&mut self) -> std::io::Result<i32> {
        if self.read_bit()? {
            return self.read::<i32>(2);
        }
        if self.read_bit()? {
            return self.read::<i32>(4);
        }
        if self.read_bit()? {
            return self.read::<i32>(10);
        }
        if self.read_bit()? {
            return self.read::<i32>(17);
        }
        self.read::<i32>(31)
    }
}

fn PlusOne(f: &mut FieldPath, _reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += 1;
    Ok(())
}

fn PlusTwo(f: &mut FieldPath, _reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += 2;
    Ok(())
}

fn PlusThree(f: &mut FieldPath, _reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += 3;
    Ok(())
}

fn PlusFour(f: &mut FieldPath, _reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += 4;
    Ok(())
}

fn PlusN(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += reader.read_fp_bitvar()? + 5;
    Ok(())
}

fn PushOneLeftDeltaZeroRightZero(
    f: &mut FieldPath,
    _reader: &mut BitReader,
) -> std::io::Result<()> {
    f.data.push(0);
    Ok(())
}

fn PushOneLeftDeltaZeroRightNonZero(
    f: &mut FieldPath,
    reader: &mut BitReader,
) -> std::io::Result<()> {
    f.data.push(reader.read_fp_bitvar()?);
    Ok(())
}

fn PushOneLeftDeltaOneRightZero(f: &mut FieldPath, _reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += 1;
    f.data.push(0);
    Ok(())
}

fn PushOneLeftDeltaOneRightNonZero(
    f: &mut FieldPath,
    reader: &mut BitReader,
) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += 1;
    f.data.push(reader.read_fp_bitvar()?);
    Ok(())
}

fn PushOneLeftDeltaNRightZero(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += reader.read_fp_bitvar()?;
    f.data.push(0);
    Ok(())
}

fn PushOneLeftDeltaNRightNonZero(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += reader.read_fp_bitvar()? + 2;
    f.data.push(reader.read_fp_bitvar()? + 1);
    Ok(())
}

fn PushOneLeftDeltaNRightNonZeroPack6Bits(
    f: &mut FieldPath,
    reader: &mut BitReader,
) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += reader.read::<i32>(3)? + 2;
    f.data.push(reader.read::<i32>(3)? + 1);
    Ok(())
}

fn PushOneLeftDeltaNRightNonZeroPack8Bits(
    f: &mut FieldPath,
    reader: &mut BitReader,
) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += reader.read::<i32>(4)? + 2;
    f.data.push(reader.read::<i32>(4)? + 1);
    Ok(())
}

fn PushTwoLeftDeltaZero(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    f.data.push(reader.read_fp_bitvar()?);
    f.data.push(reader.read_fp_bitvar()?);
    Ok(())
}

fn PushTwoLeftDeltaOne(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += 1;
    f.data.push(reader.read_fp_bitvar()?);
    f.data.push(reader.read_fp_bitvar()?);
    Ok(())
}

fn PushTwoLeftDeltaN(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += reader.read_ubitvar()? as i32 + 2;
    f.data.push(reader.read_fp_bitvar()?);
    f.data.push(reader.read_fp_bitvar()?);
    Ok(())
}

fn PushTwoPack5LeftDeltaZero(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    f.data.push(reader.read::<i32>(5)?);
    f.data.push(reader.read::<i32>(5)?);
    Ok(())
}

fn PushTwoPack5LeftDeltaOne(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += 1;
    f.data.push(reader.read::<i32>(5)?);
    f.data.push(reader.read::<i32>(5)?);
    Ok(())
}

fn PushTwoPack5LeftDeltaN(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += reader.read_ubitvar()? as i32 + 2;
    f.data.push(reader.read::<i32>(5)?);
    f.data.push(reader.read::<i32>(5)?);
    Ok(())
}

fn PushThreeLeftDeltaZero(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    f.data.push(reader.read_fp_bitvar()?);
    f.data.push(reader.read_fp_bitvar()?);
    f.data.push(reader.read_fp_bitvar()?);
    Ok(())
}

fn PushThreeLeftDeltaOne(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += 1;
    f.data.push(reader.read_fp_bitvar()?);
    f.data.push(reader.read_fp_bitvar()?);
    f.data.push(reader.read_fp_bitvar()?);
    Ok(())
}

fn PushThreeLeftDeltaN(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += reader.read_ubitvar()? as i32 + 2;
    f.data.push(reader.read_fp_bitvar()?);
    f.data.push(reader.read_fp_bitvar()?);
    f.data.push(reader.read_fp_bitvar()?);
    Ok(())
}

fn PushThreePack5LeftDeltaZero(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    f.data.push(reader.read::<i32>(5)?);
    f.data.push(reader.read::<i32>(5)?);
    f.data.push(reader.read::<i32>(5)?);
    Ok(())
}

fn PushThreePack5LeftDeltaOne(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += 1;
    f.data.push(reader.read::<i32>(5)?);
    f.data.push(reader.read::<i32>(5)?);
    f.data.push(reader.read::<i32>(5)?);
    Ok(())
}

fn PushThreePack5LeftDeltaN(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    *f.data.last_mut().unwrap() += reader.read_ubitvar()? as i32 + 2;
    f.data.push(reader.read::<i32>(5)?);
    f.data.push(reader.read::<i32>(5)?);
    f.data.push(reader.read::<i32>(5)?);
    Ok(())
}

fn PushN(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    for _ in 0..reader.read_ubitvar()? {
        f.data.push(reader.read_fp_bitvar()?);
    }
    Ok(())
}

fn PushNAndNonTopological(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    for idx in &mut f.data {
        if reader.read_bit()? {
            *idx += reader.read_signed_varint32()? + 1;
        }
    }
    for _ in 0..reader.read_ubitvar()? {
        f.data.push(reader.read_fp_bitvar()?);
    }
    Ok(())
}

fn PopOnePlusOne(f: &mut FieldPath, _reader: &mut BitReader) -> std::io::Result<()> {
    f.data.pop();
    *f.data.last_mut().unwrap() += 1;
    Ok(())
}

fn PopOnePlusN(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    f.data.pop();
    *f.data.last_mut().unwrap() += reader.read_fp_bitvar()? + 1;
    Ok(())
}

fn PopAllButOnePlusOne(f: &mut FieldPath, _reader: &mut BitReader) -> std::io::Result<()> {
    f.data.truncate(1);
    *f.data.last_mut().unwrap() += 1;
    Ok(())
}

fn PopAllButOnePlusN(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    f.data.truncate(1);
    *f.data.last_mut().unwrap() += reader.read_fp_bitvar()? + 1;
    Ok(())
}

fn PopAllButOnePlusNPack3Bits(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    f.data.truncate(1);
    *f.data.last_mut().unwrap() += reader.read::<i32>(3)? + 1;
    Ok(())
}

fn PopAllButOnePlusNPack6Bits(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    f.data.truncate(1);
    *f.data.last_mut().unwrap() += reader.read::<i32>(6)? + 1;
    Ok(())
}

fn PopNPlusOne(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    let nsize = f.data.len() - reader.read_fp_bitvar()? as usize;
    assert!(nsize < 7 && nsize > 0, "Invalid fp size for op");

    f.data.truncate(nsize);
    *f.data.last_mut().unwrap() += 1;
    Ok(())
}

fn PopNPlusN(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    let nsize = f.data.len() - reader.read_fp_bitvar()? as usize;
    assert!(nsize < 7 && nsize > 0, "Invalid fp size for op");

    f.data.truncate(nsize);
    *f.data.last_mut().unwrap() += reader.read_signed_varint32()?;
    Ok(())
}

fn PopNAndNonTopographical(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    let nsize = f.data.len() - reader.read_fp_bitvar()? as usize;
    assert!(nsize < 7 && nsize > 0, "Invalid fp size for op");

    f.data.truncate(nsize);
    for idx in &mut f.data {
        if reader.read_bit()? {
            *idx += reader.read_signed_varint32()?;
        }
    }
    Ok(())
}

fn NonTopoComplex(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    for idx in &mut f.data {
        if reader.read_bit()? {
            *idx += reader.read_signed_varint32()?;
        }
    }
    Ok(())
}

fn NonTopoPenultimatePlusOne(f: &mut FieldPath, _reader: &mut BitReader) -> std::io::Result<()> {
    assert!(f.data.len() >= 2, "Invalid fp size for op");
    let idx = f.data.len() - 2;
    f.data[idx] += 1;
    Ok(())
}

fn NonTopoComplexPack4Bits(f: &mut FieldPath, reader: &mut BitReader) -> std::io::Result<()> {
    for idx in &mut f.data {
        if reader.read_bit()? {
            *idx += reader.read::<i32>(4)? - 7;
        }
    }
    Ok(())
}

fn FieldPathEncodeFinish(f: &mut FieldPath, _reader: &mut BitReader) -> std::io::Result<()> {
    f.finished = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let a = vec![
            ("PlusOne", 0),
            ("FieldPathEncodeFinish", 2),
            ("PlusTwo", 14),
            ("PushOneLeftDeltaNRightNonZeroPack6Bits", 15),
            ("PushOneLeftDeltaOneRightNonZero", 24),
            ("PlusN", 26),
            ("PlusThree", 50),
            ("PopAllButOnePlusOne", 51),
            ("PushOneLeftDeltaNRightNonZero", 217),
            ("PushOneLeftDeltaOneRightZero", 218),
            ("PushOneLeftDeltaNRightZero", 220),
            ("PopAllButOnePlusNPack6Bits", 222),
            ("PlusFour", 223),
            ("PopAllButOnePlusN", 432),
            ("PushOneLeftDeltaNRightNonZeroPack8Bits", 438),
            ("NonTopoPenultimatePlusOne", 439),
            ("PopAllButOnePlusNPack3Bits", 442),
            ("PushNAndNonTopological", 443),
            ("NonTopoComplexPack4Bits", 866),
            ("NonTopoComplex", 1735),
            ("PushOneLeftDeltaZeroRightZero", 3469),
            ("PopOnePlusOne", 27745),
            ("PushOneLeftDeltaZeroRightNonZero", 27749),
            ("PopNAndNonTopographical", 55488),
            ("PopNPlusN", 55489),
            ("PushN", 55492),
            ("PushThreePack5LeftDeltaN", 55493),
            ("PopNPlusOne", 55494),
            ("PopOnePlusN", 55495),
            ("PushTwoLeftDeltaZero", 55496),
            ("PushThreeLeftDeltaZero", 110994),
            ("PushTwoPack5LeftDeltaZero", 110995),
            ("PushTwoLeftDeltaN", 111000),
            ("PushThreePack5LeftDeltaOne", 111001),
            ("PushThreeLeftDeltaN", 111002),
            ("PushTwoPack5LeftDeltaN", 111003),
            ("PushTwoLeftDeltaOne", 111004),
            ("PushThreePack5LeftDeltaZero", 111005),
            ("PushThreeLeftDeltaOne", 111006),
            ("PushTwoPack5LeftDeltaOne", 111007),
        ];
        for (name, mut val) in a {
            print!("({name}, vec![");
            let mut bits = vec![];
            while val > 0 {
                bits.push(val & 1);
                val >>= 1;
            }
            bits.reverse();
            print!(
                "{}",
                bits.iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            );
            println!("]),");
        }
    }
}
