mod class;
mod decoder;
mod fieldpath;
mod property;
mod send_tables;

use std::rc::Rc;

use bitstream_io::BitRead;
use tracing::trace;

use self::fieldpath::FieldPath;
use self::send_tables::{Field, Serializer};
use crate::proto::netmessages::CSVCMsg_PacketEntities;
use crate::read::ValveBitReader;
use crate::BitReader;
use crate::{Error, Result};

pub use self::class::Classes;
pub use self::property::{Object, Property};
pub use self::send_tables::SendTables;

#[derive(Default)]
pub struct Entities {
    entities: Vec<Option<Entity>>,
    /// Only used by read_props to avoid allocations.
    field_paths: Vec<FieldPath>,
}

impl Entities {
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
                        entity.read_props(&mut reader, &mut self.field_paths)?;
                    } else {
                        return Err(Error::InvalidEntityId);
                    }
                }
                (false, true) => {
                    let class_id = reader.read::<u32>(classes.class_id_bits)?;
                    let _serial = reader.read::<u32>(17)?;
                    reader.read_varuint32()?; // Don't know what this is.
                    let class = classes.class(class_id);
                    let mut entity = Entity::new(Rc::clone(&class.serializer));
                    if let Some(baseline) = &class.instance_baseline {
                        entity.read_props(&mut BitReader::new(baseline), &mut self.field_paths)?;
                    };
                    entity.read_props(&mut reader, &mut self.field_paths)?;
                    self.entities[entity_id as usize] = Some(entity);
                }
                (true, _) => {
                    self.entities[entity_id as usize] = None;
                }
            };
        }
        Ok(())
    }
}

impl std::ops::Index<usize> for Entities {
    type Output = Entity;

    /// Returns a reference to the entity with the supplied id.
    ///
    /// # Panics
    ///
    /// Panics if the id is not found.
    fn index(&self, id: usize) -> &Self::Output {
        self.entities[id].as_ref().expect("no entry for index")
    }
}

#[derive(Debug)]
pub struct Entity {
    object: Object,
    serializer: Rc<Serializer>,
}

impl Entity {
    fn new(serializer: Rc<Serializer>) -> Self {
        Self {
            object: Object::new(&serializer),
            serializer,
        }
    }

    pub fn get_property(&self, fp: &[i32]) -> (Option<&Property>, &Field, PathName) {
        let prop = self.object.properties[fp[0] as usize].as_ref();
        let field = &self.serializer.fields[fp[0] as usize];
        let name = PathName {
            items: vec![PathNameItem::Field(field.name())],
        };
        fp[1..]
            .iter()
            .fold((prop, field, name), |(prop, field, name), &i| {
                let i = i as usize;
                let is_array = matches!(field, Field::Array(_) | Field::Vector(_));
                let (prop, field) = match (prop, field) {
                    (Some(Property::Object(o)), Field::Object(f)) => {
                        (o.properties[i].as_ref(), &f.serializer.fields[i])
                    }
                    (Some(Property::Array(a)), Field::Array(f)) => {
                        (a[i].as_ref(), f.element.as_ref())
                    }
                    (Some(Property::Array(a)), Field::Vector(f)) => {
                        (a[i].as_ref(), f.element.as_ref())
                    }
                    (None, f) => (None, f),
                    (Some(p), f) => unreachable!("{p:?} {f:?}"),
                };
                let name = if is_array {
                    name.push_index(i)
                } else {
                    name.push_field(field.name())
                };
                (prop, field, name)
            })
    }

    fn property(&mut self, fp: &[i32]) -> (&mut Option<Property>, &Field) {
        let prop = &mut self.object.properties[fp[0] as usize];
        let field = &self.serializer.fields[fp[0] as usize];
        fp[1..]
            .iter()
            .fold((prop, field), |(prop, field), &i| match (prop, field) {
                (Some(Property::Object(o)), Field::Object(f)) => (
                    &mut o.properties[i as usize],
                    &f.serializer.fields[i as usize],
                ),
                (Some(Property::Array(a)), Field::Array(f)) => {
                    (&mut a[i as usize], f.element.as_ref())
                }
                (Some(Property::Array(a)), Field::Vector(f)) => {
                    (&mut a[i as usize], f.element.as_ref())
                }
                (Some(p), f) => unreachable!("{p:?} {f:?}"),
                (p, Field::Array(f)) if p.is_none() => {
                    *p = Some(Property::Array(
                        vec![None; f.size as usize].into_boxed_slice(),
                    ));
                    match p {
                        Some(Property::Array(a)) => (&mut a[i as usize], &f.element.as_ref()),
                        _ => unreachable!(),
                    }
                }
                (None, f) => unreachable!("{f:?}"),
            })
    }

    /// Read props from `reader`, creating new props or overwriting existing ones.
    fn read_props(&mut self, reader: &mut BitReader, fps: &mut Vec<FieldPath>) -> Result<()> {
        let mut fp = FieldPath::new();
        fps.clear();
        loop {
            fp.read(reader)?;
            if fp.finished {
                break;
            }
            fps.push(fp.clone());
        }
        for fp in fps {
            let (prop, field) = self.property(&fp.data);
            *prop = (field.decoder())(reader)?;

            if false {
                let (prop, field, name) = self.get_property(&fp.data);
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

enum PathNameItem {
    Field(Rc<str>),
    Index(usize),
}

pub struct PathName {
    items: Vec<PathNameItem>,
}

impl PathName {
    fn push_field(mut self, field: Rc<str>) -> Self {
        self.items.push(PathNameItem::Field(field));
        self
    }

    fn push_index(mut self, index: usize) -> Self {
        self.items.push(PathNameItem::Index(index));
        self
    }
}

impl std::fmt::Display for PathName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, item) in self.items.iter().enumerate() {
            match item {
                PathNameItem::Field(field) => {
                    if idx > 0 {
                        write!(f, ".")?
                    }
                    write!(f, "{field}")?;
                }
                PathNameItem::Index(index) => write!(f, ".{index:04}")?,
            }
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

        let mut entities = Entities::default();
        entities.read_packet_entities(testdata::packet_entities(), &classes)?;
        Ok(())
    }
}
