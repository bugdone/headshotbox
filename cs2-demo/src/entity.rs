mod class;
mod decoder;
mod fieldpath;
mod property;
mod path_name;
mod send_tables;

use std::rc::Rc;

use bitstream_io::BitRead;
use tracing::{enabled, trace, Level};

use self::fieldpath::FieldPath;
use self::path_name::PathName;
use self::send_tables::{Field, Serializer};
use crate::proto::netmessages::CSVCMsg_PacketEntities;
use crate::read::ValveBitReader;
use crate::BitReader;
use crate::{Error, Result};

pub use self::class::Classes;
pub use self::property::{Property, TreeEntity};
pub use self::send_tables::SendTables;

pub trait Entity: std::fmt::Display {
    fn serializer(&self) -> &Rc<Serializer>;
    fn get_property(&self, fp: &[i32]) -> (Option<&Property>, &Field, PathName);
    fn set_property(&mut self, fp: &[i32], value: Option<Property>);
}

pub type EntityFactory = &'static dyn Fn(Rc<Serializer>) -> Box<dyn Entity>;

pub struct EntityList {
    entities: Vec<Option<Box<dyn Entity>>>,
    entity_factory: EntityFactory,
    /// Only used by read_props to avoid allocations.
    field_paths: Vec<FieldPath>,
}

impl EntityList {
    pub fn new(entity_factory: EntityFactory) -> Self {
        Self {
            entities: Default::default(),
            entity_factory,
            field_paths: Vec::with_capacity(512),
        }
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn read_packet_entities(
        &mut self,
        msg: CSVCMsg_PacketEntities,
        classes: &Classes,
    ) -> Result<()> {
        let mut next_entity_id = 0;
        let mut reader = BitReader::new(msg.entity_data());
        for _ in 0..msg.updated_entries() {
            let entity_id = next_entity_id + reader.read_ubitvar()?;
            next_entity_id = entity_id + 1;
            let remove = reader.read_bit()?;
            let new = reader.read_bit()?;
            match (remove, new) {
                (false, false) => {
                    trace!("Update entity {entity_id}");
                    if let Some(entity) = self.entities[entity_id as usize].as_mut() {
                        Self::read_props(&mut reader, entity.as_mut(), &mut self.field_paths)?;
                    } else {
                        return Err(Error::InvalidEntityId);
                    }
                }
                (false, true) => {
                    let class_id = reader.read::<u32>(classes.class_id_bits)?;
                    let _serial = reader.read::<u32>(17)?;
                    reader.read_varuint32()?; // Don't know what this is.
                    let class = classes.class(class_id);
                    trace!("Create entity {entity_id} {}", class.serializer.name);
                    let mut entity = (self.entity_factory)(Rc::clone(&class.serializer));
                    if let Some(baseline) = &class.instance_baseline {
                        Self::read_props(
                            &mut BitReader::new(baseline),
                            entity.as_mut(),
                            &mut self.field_paths,
                        )?;
                        trace!("Baseline for entity {entity_id} done");
                    };
                    Self::read_props(&mut reader, entity.as_mut(), &mut self.field_paths)?;
                    if self.entities.len() <= entity_id as usize {
                        self.entities.resize_with(entity_id as usize + 1, || None);
                    }
                    self.entities[entity_id as usize] = Some(entity);
                }
                (true, _) => {
                    trace!("Delete entity {entity_id}");
                    self.entities[entity_id as usize] = None;
                }
            };
        }
        Ok(())
    }

    /// Read props from `reader`, creating new props or overwriting existing ones.
    fn read_props(
        reader: &mut BitReader,
        entity: &mut dyn Entity,
        fps: &mut Vec<FieldPath>,
    ) -> Result<()> {
        let mut fp = FieldPath::new();
        fps.clear();
        loop {
            fp.read(reader)?;
            if fp.len() == 0 {
                break;
            }
            fps.push(fp.clone());
        }
        let serializer = Rc::clone(entity.serializer());
        for fp in fps {
            let field = get_field(serializer.as_ref(), fp.data());
            entity.set_property(fp.data(), field.decoder().decode(reader)?);

            if enabled!(Level::TRACE) {
                let (prop, field, name) = entity.get_property(fp.data());
                match field {
                    Field::Value(_) | Field::Array(_) | Field::Vector(_) => {
                        trace!("{fp} {}: {} = {}", name, field.ctype(), prop.unwrap())
                    }
                    Field::Object(_) => {
                        trace!("{fp} {}: {} = {}", name, field.ctype(), prop.is_some())
                    }
                }
            }
        }
        Ok(())
    }
}

fn get_field<'a>(serializer: &'a Serializer, fp: &[i32]) -> &'a Field {
    let field = &serializer.fields[fp[0] as usize];
    fp[1..].iter().fold(field, |field, &i| match field {
        Field::Object(f) => &f.serializer.fields[i as usize],
        Field::Array(f) => f.element.as_ref(),
        Field::Vector(f) => f.element.as_ref(),
        Field::Value(_) => unreachable!(),
    })
}

impl std::ops::Index<usize> for EntityList {
    type Output = dyn Entity;

    /// Returns a reference to the entity with the supplied id.
    ///
    /// # Panics
    ///
    /// Panics if the id is not found.
    fn index(&self, id: usize) -> &Self::Output {
        self.entities[id]
            .as_ref()
            .expect("no entry for index")
            .as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata;

    #[test]
    fn test() -> Result<()> {
        let send_tables = SendTables::try_new(testdata::send_tables())?;
        let mut classes = Classes::try_new(testdata::class_info(), send_tables)?;
        for table in testdata::string_tables().tables {
            if table.table_name() == "instancebaseline" {
                let items = table
                    .items
                    .into_iter()
                    .map(|mut e| (e.take_str(), e.take_data()))
                    .collect();
                classes.update_instance_baselines(items);
            }
        }

        let mut entities = EntityList::new(&TreeEntity::factory);
        entities.read_packet_entities(testdata::packet_entities(), &classes)?;
        Ok(())
    }
}
