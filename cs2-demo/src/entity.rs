mod class;
mod decoder;
mod fieldpath;
mod property;
mod send_tables;

use std::rc::Rc;

use bitstream_io::BitRead;
use demo_format::read::ValveBitReader;
use demo_format::BitReader;

use self::fieldpath::FieldPath;
use self::send_tables::{Field, Serializer};
use crate::proto::netmessages::CSVCMsg_PacketEntities;
use crate::{Error, Result};

pub use self::class::Classes;
pub use self::property::{Properties, Property};
pub use self::send_tables::SendTables;

#[derive(Default)]
pub struct Entities {
    entities: Vec<Option<Entity>>,
}

impl Entities {
    pub(crate) fn read_packet_entities(
        &mut self,
        msg: CSVCMsg_PacketEntities,
        classes: &Classes,
    ) -> Result<()> {
        let mut next_entity_id = 0;
        let mut reader = BitReader::new(msg.entity_data());
        let max_entries = msg.max_entries() as usize;
        self.entities.resize_with(max_entries, || None);
        for _ in 0..msg.updated_entries() {
            let entity_id = next_entity_id + reader.read_ubitvar()?;
            next_entity_id = entity_id + 1;
            let remove = reader.read_bit()?;
            let new = reader.read_bit()?;
            match (remove, new) {
                (false, false) => {
                    if let Some(entity) = self.entities[entity_id as usize].as_mut() {
                        entity.read_props(&mut reader)?;
                    } else {
                        return Err(Error::InvalidEntityId);
                    }
                }
                (false, true) => {
                    let class_id = reader.read::<u32>(classes.class_id_bits)?;
                    let _serial = reader.read::<u32>(17)?;
                    reader.read_varint32()?; // Don't know what this is.

                    // TODO: read baseline
                    println!("== new entity {entity_id} class={class_id}");
                    let serializer = Rc::clone(&classes.class(class_id).serializer);
                    let mut entity = Entity::new(serializer);
                    entity.read_props(&mut reader)?;
                    self.entities[entity_id as usize] = Some(entity);
                }
                (true, _) => {
                    todo!()
                }
            };
        }
        Ok(())
    }
}

pub struct Entity {
    serializer: Rc<Serializer>,
    properties: Properties,
}

impl Entity {
    fn new(serializer: Rc<Serializer>) -> Self {
        let properties = vec![None; serializer.fields.len()];
        Self {
            serializer,
            properties,
        }
    }

    fn property(&mut self, fp: &[i32]) -> (&mut Option<Property>, &Field) {
        let mut properties = &mut self.properties;
        let mut fields = &self.serializer.fields;
        for i in 0..fp.len() {
            let prop = &mut properties[fp[i] as usize];
            let field = &fields[fp[i] as usize];
            println!("{field:?}");
            match &field.serializer {
                Some(ser) => {
                    let prop =
                        prop.get_or_insert_with(|| Property::Object(vec![None; ser.fields.len()]));
                    match prop {
                        Property::Object(o) => {
                            properties = o;
                            fields = &ser.fields;
                        }
                        _ => unreachable!(),
                    }
                }
                None => {
                    assert!(i == fp.len() - 1);
                    return (prop, field);
                }
            }
        }
        unreachable!()
    }

    /// Read props from `reader`, creating new props or overwriting existing ones.
    fn read_props(&mut self, reader: &mut BitReader) -> Result<()> {
        let mut fp = FieldPath::new();
        let mut fps = Vec::with_capacity(256);
        loop {
            fp.read(reader)?;
            if fp.finished {
                break;
            }
            fps.push(fp.clone());
        }
        for fp in fps {
            let (prop, field) = self.property(&fp.data);
            println!("{fp} {}: {}", field.var_name, field.var_type);
            *prop = Some((field.decoder)(reader)?);
            println!("{fp} {} = {:?}", field.var_name, prop.as_ref().unwrap());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata;

    #[test]
    fn test() -> Result<()> {
        let send_tables = SendTables::try_new(testdata::send_tables())?;
        let classes = Classes::try_new(testdata::class_info(), send_tables)?;
        let mut entities = Entities::default();
        entities.read_packet_entities(testdata::packet_entities(), &classes)?;
        Ok(())
    }
}
