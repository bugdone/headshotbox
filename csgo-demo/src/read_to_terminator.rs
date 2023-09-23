use bitstream_io::BitRead;
use demo_format::BitReader;
use protobuf::CodedInputStream;
use std::io;

trait ReadByte {
    fn read_byte(&mut self) -> io::Result<u8>;
}

impl<'a> ReadByte for BitReader<'a> {
    fn read_byte(&mut self) -> io::Result<u8> {
        self.read::<u8>(8)
    }
}

impl<'a> ReadByte for CodedInputStream<'a> {
    fn read_byte(&mut self) -> io::Result<u8> {
        self.read_raw_byte().map_err(Into::into)
    }
}

pub(crate) trait ReadToTerminator {
    /// Read bytes until a null terminator (`\0`) is found. Bytes are then
    /// converted to a String and returned.
    ///
    /// # Errors
    ///
    /// If `limit` is reached and no null terminator has been found, an error
    /// is returned.
    fn read_string_to_terminator(&mut self, limit: usize) -> io::Result<String>;
}

impl<RB: ReadByte> ReadToTerminator for RB {
    fn read_string_to_terminator(&mut self, limit: usize) -> io::Result<String> {
        let mut bytes: Vec<u8> = Vec::with_capacity(limit.min(256));

        for _ in 0..limit {
            let c = self.read_byte()?;
            if c == 0 {
                return Ok(String::from_utf8_lossy(&bytes).into_owned());
            }
            bytes.push(c);
        }

        Err(io::Error::new(
            io::ErrorKind::Other,
            "limit has been reached without finding a null terminator",
        ))
    }
}
