/// https://developer.valvesoftware.com/wiki/Networking_Entities
mod serverclass;

use anyhow::{bail, Context};
use bitstream_io::BitRead;
use csgo_demo_parser::messages::CSVCMsg_PacketEntities;
use demo_format::read::ValveBitReader;
use demo_format::BitReader;
use demo_format::Tick;
use serverclass::ServerClass;
pub(crate) use serverclass::ServerClasses;
use std::io;
use std::rc::Rc;

const MAX_ENTITIES: u32 = 2048;

type ServerPropP = Rc<dyn Fn(&str, &str) -> TrackProp>;
type PropChange = Rc<dyn Fn(&Entity, Tick, &PropValue)>;

pub enum TrackProp {
    // Don't track the prop, for performance reasons.
    No,
    // Track the prop value.
    Value,
    // Track the prop and run a callback whenever its value changes.
    Changes(PropChange),
}

#[derive(Default)]
pub struct EntityConfig {
    /// Can be used to specify which entity properties should be tracked.
    ///
    /// If `None`, all properties are tracked.
    pub tracked_props: Option<ServerPropP>,
}

pub type EntityId = u16;

pub struct Entities<'a> {
    server_classes: &'a ServerClasses,
    entities: Vec<Option<Entity<'a>>>,
    /// Only used by read_props to avoid allocations.
    field_indices: Vec<i32>,
}

impl<'a> Entities<'a> {
    pub(crate) fn new(server_classes: &'a ServerClasses) -> Self {
        Self {
            server_classes,
            entities: (0..MAX_ENTITIES).map(|_| None).collect(),
            field_indices: Vec::with_capacity(512),
        }
    }

    pub fn get(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(id as usize)?.as_ref()
    }

    pub fn read_packet_entities(
        &mut self,
        msg: CSVCMsg_PacketEntities,
        tick: Tick,
    ) -> anyhow::Result<()> {
        let mut next_entity_id = 0;
        let mut reader = BitReader::new(msg.entity_data());
        for _ in 0..msg.updated_entries() {
            let entity_id = next_entity_id + reader.read_ubitvar()?;
            next_entity_id = entity_id + 1;
            if entity_id >= MAX_ENTITIES {
                bail!("invalid entity_id");
            }
            let remove = reader.read_bit()?;
            let new = reader.read_bit()?;
            match (remove, new) {
                (false, false) => {
                    if let Some(entity) = self.entities[entity_id as usize].as_mut() {
                        entity.read_props(&mut reader, &mut self.field_indices, tick)?;
                    } else {
                        bail!("entity id not found");
                    }
                }
                (false, true) => {
                    let class_id = reader.read::<u32>(self.server_classes.bits)?;
                    let class = &self
                        .server_classes
                        .server_classes
                        .get(class_id as usize)
                        .ok_or_else(|| anyhow::anyhow!("class id not found"))?;
                    // Discard serial_num.
                    reader.read::<u32>(10)?;
                    let mut entity = Entity::new(entity_id as EntityId, class);
                    entity.read_props(&mut reader, &mut self.field_indices, tick)?;
                    self.entities[entity_id as usize] = Some(entity);
                }
                (true, _) => {
                    if !msg.is_delta() {
                        bail!("Entities should not be deleted in a full update");
                    }
                    if self.entities[entity_id as usize].take().is_none() {
                        bail!("Tried to remove an entity which doesn't exist")
                    }
                }
            };
        }
        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Scalar {
    I32(i32),
    F32(f32),
    String(String),
    I64(i64),
    Vector(Vector),
}

#[derive(Debug)]
pub enum PropValue {
    Scalar(Scalar),
    Array(Vec<Scalar>),
}

pub struct Entity<'a> {
    class: &'a ServerClass,
    pub id: EntityId,
    pub(crate) props: Vec<Option<PropValue>>,
}

impl std::fmt::Display for Entity<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.class.name)?;
        for (d, p) in std::iter::zip(self.class.props.iter(), self.props.iter()) {
            if let Some(p) = p {
                writeln!(f, "  {}={:?}", d.name, p)?
            }
        }
        std::fmt::Result::Ok(())
    }
}

impl Entity<'_> {
    pub fn get_prop(&self, prop_name: &str) -> Option<&PropValue> {
        let idx = self
            .class
            .props
            .iter()
            .position(|pd| pd.name == prop_name)?;
        self.props.get(idx)?.as_ref()
    }

    fn new(id: EntityId, class: &ServerClass) -> Entity {
        let props = class.props.iter().map(|_| None).collect();
        Entity { id, class, props }
    }

    /// Read props from `reader`, creating new props or overwriting existing ones.
    fn read_props(
        &mut self,
        reader: &mut BitReader,
        field_indices: &mut Vec<i32>,
        tick: Tick,
    ) -> anyhow::Result<()> {
        let new_way = reader.read_bit()?;
        field_indices.clear();
        let mut index = -1;
        while let Some(val) = read_entity_field_index(reader, index, new_way)? {
            index = val;
            field_indices.push(index);
            if field_indices.len() > 20000 {
                bail!("found too many entity field indices, probably corrupt demo")
            }
        }
        for i in field_indices {
            let descriptor = self
                .class
                .props
                .get(*i as usize)
                .context("invalid prop index")?;
            match &descriptor.track {
                TrackProp::No => descriptor.skip(reader)?,
                TrackProp::Value => self.props[*i as usize] = Some(descriptor.decode(reader)?),
                TrackProp::Changes(prop_changed) => {
                    let value = descriptor.decode(reader)?;
                    prop_changed(self, tick, &value);
                    self.props[*i as usize] = Some(value);
                }
            }
        }
        Ok(())
    }
}

fn read_entity_field_index(
    reader: &mut BitReader,
    last_index: i32,
    new_way: bool,
) -> io::Result<Option<i32>> {
    if new_way && reader.read_bit()? {
        return Ok(Some(last_index + 1));
    }

    let offset = if new_way && reader.read_bit()? {
        reader.read::<u32>(3)?
    } else {
        let tmp = reader.read::<u32>(7)?;
        match tmp & (32 | 64) {
            32 => reader.read::<u32>(2)? << 5 | (tmp & 0x1F),
            64 => reader.read::<u32>(4)? << 5 | (tmp & 0x1F),
            96 => reader.read::<u32>(7)? << 5 | (tmp & 0x1F),
            _ => tmp,
        }
    };
    if offset == 0xFFF {
        Ok(None)
    } else {
        Ok(Some(last_index + 1 + offset as i32))
    }
}
