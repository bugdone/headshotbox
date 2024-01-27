use std::collections::HashMap;
use std::rc::Rc;

use protobuf::CodedInputStream;

use crate::proto::demo::CDemoSendTables;
use crate::proto::netmessages::CSVCMsg_FlattenedSerializer;
use crate::{Error, Result};

use super::decoder::{Decoder, DecoderCache};

#[derive(Debug, Default)]
pub struct SendTables {
    pub(super) serializers: Vec<Rc<Serializer>>,
}

#[derive(Debug)]
pub(super) struct Serializer {
    pub(super) name: String,
    pub(super) fields: Vec<Field>,
}

impl std::fmt::Display for Serializer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "class {}", self.name)?;
        for field in &self.fields {
            writeln!(f, "  {}: {}", field.var_name, field.var_type)?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(super) struct Field {
    pub(super) var_name: Rc<str>,
    pub(super) var_type: Rc<str>,
    pub(super) serializer: Option<Rc<Serializer>>,
    pub(super) decoder: Decoder,
}

impl std::fmt::Debug for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("Field")
            .field("var_name", &self.var_name)
            .field("var_type", &self.var_type)
            .field("serializer", &self.serializer.as_ref().map(|s| s.name.as_str()))
            .finish()
    }
}

impl SendTables {
    pub fn try_new(msg: CDemoSendTables) -> Result<Self> {
        let mut data = msg.data();
        let fs: CSVCMsg_FlattenedSerializer = CodedInputStream::new(&mut data)
            .read_message()
            .or(Err(Error::InvalidSendTables))?;
        let data = SentTableData::new(fs);
        SendTableBuilder::new(&data).build()
    }
}

struct SentTableData {
    fs: CSVCMsg_FlattenedSerializer,
    symbols: Vec<Rc<str>>,
    /// Maps `(serializer_name_sym, serializer_version)` to indices into `serializers`.
    by_name_ver: HashMap<(i32, i32), i32>,
}

impl SentTableData {
    fn new(fs: CSVCMsg_FlattenedSerializer) -> Self {
        let by_name_ver = fs
            .serializers
            .iter()
            .enumerate()
            .map(|(i, s)| {
                (
                    (
                        s.serializer_name_sym.unwrap(),
                        s.serializer_version.unwrap(),
                    ),
                    i as i32,
                )
            })
            .collect();
        let symbols = fs.symbols.iter().map(|s| Rc::from(s.as_str())).collect();
        Self {
            fs,
            symbols,
            by_name_ver,
        }
    }
}

struct SendTableBuilder<'a> {
    data: &'a SentTableData,
    serializers: Vec<Option<Rc<Serializer>>>,
    decoder_cache: DecoderCache,
}

impl<'a> SendTableBuilder<'a> {
    fn new(data: &'a SentTableData) -> Self {
        let serializers = vec![None; data.fs.serializers.len()];
        let decoder_cache = DecoderCache::new();
        Self {
            data,
            serializers,
            decoder_cache,
        }
    }

    fn build(mut self) -> Result<SendTables> {
        for si in 0..self.serializers.len() as i32 {
            self.serializer(si);
        }
        let serializers = self
            .serializers
            .into_iter()
            .map(|s| s.ok_or(Error::InvalidSendTables))
            .collect::<Result<Vec<_>>>()?;
        Ok(SendTables { serializers })
    }

    fn serializer(&mut self, si: i32) -> Rc<Serializer> {
        if self.serializers[si as usize].is_some() {
            return Rc::clone(self.serializers[si as usize].as_ref().unwrap());
        }
        let s = &self.data.fs.serializers[si as usize];
        let name = self.data.symbols[s.serializer_name_sym.unwrap() as usize].to_string();
        let mut fields = Vec::with_capacity(s.fields_index.len());
        for &fi in &s.fields_index {
            fields.push(self.field(fi));
        }
        let s = Rc::new(Serializer { name, fields });
        self.serializers[si as usize] = Some(Rc::clone(&s));
        s
    }

    fn field(&mut self, fi: i32) -> Field {
        let fsf = &self.data.fs.fields[fi as usize];
        let var_name = Rc::clone(&self.data.symbols[fsf.var_name_sym.unwrap() as usize]);
        let var_type = Rc::clone(&self.data.symbols[fsf.var_type_sym.unwrap() as usize]);
        let encoder = match var_name.as_ref() {
            "m_flSimulationTime" | "m_flAnimTime" => Some("simtime"),
            _ => fsf
                .var_encoder_sym
                .map(|e| self.data.symbols[e as usize].as_ref()),
        };
        let decoder = self.decoder_cache.make_decoder(
            var_type.as_ref(),
            encoder,
            fsf.bit_count(),
            fsf.low_value.unwrap_or(0f32),
            fsf.high_value.unwrap_or(1f32),
            fsf.encode_flags(),
        );
        let serializer = match (fsf.field_serializer_name_sym, fsf.field_serializer_version) {
            (Some(name), Some(version)) => {
                let si = self.data.by_name_ver.get(&(name, version)).cloned();
                si.map(|si| self.serializer(si))
            }
            _ => None,
        };
        Field {
            var_name,
            var_type,
            serializer,
            decoder,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata;

    #[test]
    fn test() {
        SendTables::try_new(testdata::send_tables()).unwrap();
    }

    #[ignore]
    #[test]
    fn dump_serializers() {
        // Run with `cargo test -p cs2-demo entity::send_tables::tests::dump_serializers -- --ignored --nocapture`
        let st = SendTables::try_new(testdata::send_tables()).unwrap();
        for s in st.serializers {
            println!("{}", s);
        }
    }

    #[ignore]
    #[test]
    fn dump_fields() {
        let st = testdata::send_tables();
        let fs: CSVCMsg_FlattenedSerializer = CodedInputStream::new(&mut st.data())
            .read_message()
            .or(Err(Error::InvalidSendTables))
            .unwrap();
        for f in fs.fields {
            println!("{}", f);
        }
    }
}
