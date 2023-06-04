/// https://developer.valvesoftware.com/wiki/Networking_Entities
mod serverclass;

use crate::BitReader;
use crate::Tick;
use anyhow::{bail, Context};
use bitstream_io::BitRead;
use csgo_demo_parser::messages::CSVCMsg_PacketEntities;
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

enum CoordType {
    None,
    LowPrecision,
    Integral,
}

const COORD_INTEGER_BITS: u32 = 14;
const COORD_INTEGER_BITS_MP: u32 = 11;
const COORD_FRACTIONAL_BITS: u32 = 5;
const COORD_FRACTIONAL_BITS_LOWPRECISION: u32 = 3;
const COORD_RESOLUTION: f32 = 1_f32 / (1 << COORD_FRACTIONAL_BITS) as f32;
const COORD_RESOLUTION_LOWPRECISION: f32 = 1_f32 / (1 << COORD_FRACTIONAL_BITS_LOWPRECISION) as f32;

#[inline]
fn read_coord_fraction(reader: &mut BitReader) -> io::Result<f32> {
    Ok(reader.read::<u32>(COORD_FRACTIONAL_BITS)? as f32 * COORD_RESOLUTION)
}

#[inline]
fn read_coord_fraction_low_precision(reader: &mut BitReader) -> io::Result<f32> {
    Ok(
        reader.read::<u32>(COORD_FRACTIONAL_BITS_LOWPRECISION)? as f32
            * COORD_RESOLUTION_LOWPRECISION,
    )
}

trait EntityReader {
    fn read_ubitvar(&mut self) -> io::Result<u32>;
    fn read_varint32(&mut self) -> io::Result<u32>;
    fn read_signed_varint32(&mut self) -> io::Result<i32>;

    fn read_coord(&mut self) -> io::Result<f32>;
    fn skip_coord(&mut self) -> io::Result<()>;

    fn read_coord_mp(&mut self, coord_type: CoordType) -> io::Result<f32>;
    fn skip_coord_mp(&mut self, coord_type: CoordType) -> io::Result<()>;

    fn read_cell_coord(&mut self, int_bits: u32, coord_type: CoordType) -> io::Result<f32>;
    fn skip_cell_coord(&mut self, int_bits: u32, coord_type: CoordType) -> io::Result<()>;

    fn read_normal(&mut self) -> io::Result<f32>;
    fn skip_normal(&mut self) -> io::Result<()>;
}

impl<'a> EntityReader for BitReader<'a> {
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

    fn read_varint32(&mut self) -> io::Result<u32> {
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

    fn read_signed_varint32(&mut self) -> io::Result<i32> {
        Ok(zigzag_decode(self.read_varint32()?))
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

    fn skip_coord(&mut self) -> io::Result<()> {
        let int_bits = self.read_bit()? as u32 * COORD_INTEGER_BITS;
        let fract_bits = self.read_bit()? as u32 * COORD_FRACTIONAL_BITS;
        let bits = int_bits + fract_bits + if int_bits + fract_bits > 0 { 1 } else { 0 };
        self.skip(bits)
    }

    fn read_coord_mp(&mut self, coord_type: CoordType) -> io::Result<f32> {
        let int_bits = if self.read_bit()? {
            COORD_INTEGER_BITS_MP
        } else {
            COORD_INTEGER_BITS
        };
        let has_int = self.read_bit()?;
        let negative = match coord_type {
            CoordType::Integral if !has_int => false,
            _ => self.read_bit()?,
        };
        let int = if has_int {
            self.read::<u32>(int_bits)? + 1
        } else {
            0
        };
        let fract = match coord_type {
            CoordType::None => read_coord_fraction(self)?,
            CoordType::LowPrecision => read_coord_fraction_low_precision(self)?,
            CoordType::Integral => 0_f32,
        };
        let abs = int as f32 + fract;
        Ok(if negative { -abs } else { abs })
    }

    fn skip_coord_mp(&mut self, coord_type: CoordType) -> io::Result<()> {
        let mut int_bits = if self.read_bit()? {
            COORD_INTEGER_BITS_MP
        } else {
            COORD_INTEGER_BITS
        };
        let has_int = self.read_bit()?;
        if !has_int {
            int_bits = 0;
        }
        let bits = match coord_type {
            CoordType::None => int_bits + 1 + COORD_FRACTIONAL_BITS,
            CoordType::LowPrecision => int_bits + 1 + COORD_FRACTIONAL_BITS_LOWPRECISION,
            CoordType::Integral => int_bits + has_int as u32,
        };
        self.skip(bits)
    }

    fn read_cell_coord(&mut self, int_bits: u32, coord_type: CoordType) -> io::Result<f32> {
        let int = self.read::<u32>(int_bits)? as f32;
        let val = match coord_type {
            CoordType::None => int + read_coord_fraction(self)?,
            CoordType::LowPrecision => int + read_coord_fraction_low_precision(self)?,
            CoordType::Integral => int,
        };
        Ok(val)
    }

    fn skip_cell_coord(&mut self, int_bits: u32, coord_type: CoordType) -> io::Result<()> {
        match coord_type {
            CoordType::None => self.skip(int_bits + COORD_FRACTIONAL_BITS),
            CoordType::LowPrecision => self.skip(int_bits + COORD_FRACTIONAL_BITS_LOWPRECISION),
            CoordType::Integral => self.skip(int_bits),
        }
    }

    fn read_normal(&mut self) -> io::Result<f32> {
        let sign = self.read_bit()?;
        let fract = self.read::<u32>(11)? as f32;
        let abs = fract * 1_f32 / ((1 << 11) - 1) as f32;
        Ok(if sign { -abs } else { abs })
    }

    fn skip_normal(&mut self) -> io::Result<()> {
        self.skip(12)
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

    #[test]
    fn varint() {
        let mut read = BitReader::new(&[0x01]);
        assert_eq!(read.read_varint32().unwrap(), 1);

        let mut read = BitReader::new(&[0x81, 0x23]);
        assert_eq!(read.read_varint32().unwrap(), 4481);

        let mut read = BitReader::new(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(read.read_varint32().unwrap(), 4294967295);
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
