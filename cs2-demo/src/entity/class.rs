use std::collections::HashMap;
use std::rc::Rc;

use super::send_tables::Serializer;
use super::SendTables;
use crate::proto::demo::{cdemo_class_info, CDemoClassInfo};
use crate::{Error, Result};

type ClassId = u32;

#[derive(Debug)]
pub(super) struct Class {
    class_id: ClassId,
    pub(super) serializer: Rc<Serializer>,
}

impl Class {
    fn try_new(
        msg: &cdemo_class_info::Class_t,
        serializers: &HashMap<String, Rc<Serializer>>,
    ) -> Result<Self> {
        let name = msg.network_name.as_ref().ok_or(Error::MissingClassName)?;
        let class_id = msg.class_id.ok_or(Error::MissingClassId)? as ClassId;
        Ok(Self {
            class_id,
            serializer: Rc::clone(&serializers[&name.clone()]),
        })
    }
}

#[derive(Debug, Default)]
pub struct Classes {
    classes: Vec<Class>,
    pub(super) class_id_bits: u32,
}

impl Classes {
    pub fn try_new(msg: CDemoClassInfo, send_tables: Rc<SendTables>) -> Result<Self> {
        let serializers = send_tables
            .serializers
            .iter()
            .map(|s| (s.name.clone(), Rc::clone(s)))
            .collect::<HashMap<_, _>>();
        let classes = msg
            .classes
            .iter()
            .map(|m| Class::try_new(m, &serializers))
            .collect::<Result<Vec<_>>>()?;
        for (i, c) in classes.iter().enumerate() {
            if i as ClassId != c.class_id {
                return Err(Error::SkippedClassId);
            }
        }
        let class_id_bits = u32::BITS - (classes.len() as u32).leading_zeros();
        Ok(Classes {
            classes,
            class_id_bits,
        })
    }

    pub(super) fn class(&self, class_id: ClassId) -> &Class {
        &self.classes[class_id as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata;

    #[test]
    fn test() {
        let send_tables = SendTables::try_new(testdata::send_tables()).unwrap();
        Classes::try_new(testdata::class_info(), Rc::new(send_tables)).unwrap();
    }
}
