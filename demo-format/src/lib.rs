pub mod read;

pub type Tick = i32;
pub type BitReader<'a> = bitstream_io::BitReader<&'a [u8], bitstream_io::LittleEndian>;
