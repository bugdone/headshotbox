use crate::entity::{BitReader, PropValue, Scalar, ValveBitReader, Vector};
use crate::proto::netmessages::{
    csvcmsg_class_info, csvcmsg_send_table::Sendprop_t, CSVCMsg_SendTable,
};
use crate::read::CoordType;
use crate::{num_bits, DataTables, Error, Result};
use bitstream_io::BitRead;
use std::collections::HashMap;
use std::io;
use std::string::FromUtf8Error;

use super::TrackProp;

pub struct ServerClass {
    pub name: String,
    pub props: Vec<PropDescriptor>,
}

impl ServerClass {
    fn new(name: &str, props: Vec<PropDescriptor>) -> ServerClass {
        ServerClass {
            name: name.into(),
            props,
        }
    }
}

pub struct ServerClasses {
    pub bits: u32,
    pub server_classes: Vec<ServerClass>,
}

impl ServerClasses {
    pub fn try_new(data_tables: DataTables) -> Result<Self> {
        let bits = num_bits(data_tables.server_classes().len() as u32);
        Ok(Self {
            bits,
            server_classes: ServerClassesParser::parse(
                data_tables.send_tables(),
                data_tables.server_classes(),
            )?,
        })
    }
}

#[derive(Debug, PartialEq)]
struct IntPropDescriptor {
    num_bits: u32,
    varint: bool,
    unsigned: bool,
}

impl IntPropDescriptor {
    fn from(sendprop: &Sendprop_t) -> Self {
        Self {
            num_bits: sendprop.num_bits() as u32,
            varint: sendprop.flags() & SPROP_VARINT != 0,
            unsigned: sendprop.flags() & SPROP_UNSIGNED != 0,
        }
    }
}

#[derive(Debug, PartialEq)]
enum FloatEncoding {
    Scaled,
    Coord,
    CoordMP,
    CoordMPLowPrecision,
    CoordMPIntegral,
    NoScale,
    Normal,
    CellCoord,
    CellCoordLowPrecision,
    CellCoordIntegral,
}

impl FloatEncoding {
    fn from_flags(flags: i32) -> FloatEncoding {
        if flags & (1 << 1) != 0 {
            FloatEncoding::Coord
        } else if flags & (1 << 12) != 0 {
            FloatEncoding::CoordMP
        } else if flags & (1 << 13) != 0 {
            FloatEncoding::CoordMPLowPrecision
        } else if flags & (1 << 14) != 0 {
            FloatEncoding::CoordMPIntegral
        } else if flags & (1 << 2) != 0 {
            FloatEncoding::NoScale
        } else if flags & (1 << 5) != 0 {
            FloatEncoding::Normal
        } else if flags & (1 << 15) != 0 {
            FloatEncoding::CellCoord
        } else if flags & (1 << 16) != 0 {
            FloatEncoding::CellCoordLowPrecision
        } else if flags & (1 << 17) != 0 {
            FloatEncoding::CellCoordIntegral
        } else {
            FloatEncoding::Scaled
        }
    }
}

#[derive(Debug, PartialEq)]
struct FloatPropDescriptor {
    num_bits: u32,
    low_value: f32,
    high_value: f32,
    special: FloatEncoding,
}

impl FloatPropDescriptor {
    fn from(sendprop: &Sendprop_t) -> Self {
        Self {
            num_bits: sendprop.num_bits() as u32,
            low_value: sendprop.low_value(),
            high_value: sendprop.high_value(),
            special: FloatEncoding::from_flags(sendprop.flags()),
        }
    }
}

#[derive(Debug)]
struct ArrayPropDescriptor {
    num_elements: u32,
    elem_type: ScalarPropDescriptor,
}

impl ArrayPropDescriptor {
    fn try_new(sendprop: &Sendprop_t, array_element: &Sendprop_t) -> Result<Self> {
        Ok(Self {
            num_elements: sendprop.num_elements() as u32,
            elem_type: ScalarPropDescriptor::try_new(array_element)?,
        })
    }
}

pub struct PropDescriptor {
    pub name: String,
    pub track: TrackProp,
    type_: PropDescriptorType,
}

impl PropDescriptor {
    fn new(name: &str, type_: PropDescriptorType) -> Self {
        Self {
            name: name.into(),
            track: TrackProp::Value,
            type_,
        }
    }

    pub(crate) fn decode(&self, reader: &mut BitReader) -> std::io::Result<PropValue> {
        match &self.type_ {
            PropDescriptorType::Scalar(s) => Ok(PropValue::Scalar(self.decode_scalar(s, reader)?)),
            PropDescriptorType::Array(a) => Ok(PropValue::Array(self.decode_array(a, reader)?)),
        }
    }

    pub(crate) fn skip(&self, reader: &mut BitReader) -> std::io::Result<()> {
        match &self.type_ {
            PropDescriptorType::Scalar(s) => self.skip_scalar(s, reader),
            PropDescriptorType::Array(a) => self.skip_array(a, reader),
        }
    }

    fn decode_scalar(
        &self,
        type_: &ScalarPropDescriptor,
        reader: &mut BitReader,
    ) -> std::io::Result<Scalar> {
        match type_ {
            ScalarPropDescriptor::Int(int) => Ok(Scalar::I32(self.decode_int(int, reader)?)),
            ScalarPropDescriptor::Float(f) => Ok(Scalar::F32(self.decode_float(f, reader)?)),
            ScalarPropDescriptor::Vector(f, unit) => {
                Ok(Scalar::Vector(self.decode_vector(f, *unit, reader)?))
            }
            ScalarPropDescriptor::VectorXY(f) => {
                Ok(Scalar::Vector(self.decode_vector_xy(f, reader)?))
            }
            ScalarPropDescriptor::String => Ok(Scalar::String(self.decode_string(reader)?)),
            ScalarPropDescriptor::Int64 => todo!(),
        }
    }

    fn skip_scalar(
        &self,
        type_: &ScalarPropDescriptor,
        reader: &mut BitReader,
    ) -> std::io::Result<()> {
        match type_ {
            ScalarPropDescriptor::Int(int) => Ok(self.skip_int(int, reader)?),
            ScalarPropDescriptor::Float(f) => Ok(self.skip_float(f, reader)?),
            ScalarPropDescriptor::Vector(f, unit) => Ok(self.skip_vector(f, *unit, reader)?),
            ScalarPropDescriptor::VectorXY(f) => Ok(self.skip_vector_xy(f, reader)?),
            ScalarPropDescriptor::String => self.skip_string(reader),
            ScalarPropDescriptor::Int64 => todo!(),
        }
    }

    fn decode_array(
        &self,
        type_: &ArrayPropDescriptor,
        reader: &mut BitReader,
    ) -> std::io::Result<Vec<Scalar>> {
        let len_bits = num_bits(type_.num_elements);
        let len = reader.read::<u32>(len_bits)?;
        let mut array = Vec::with_capacity(len as usize);
        for _ in 0..len {
            array.push(self.decode_scalar(&type_.elem_type, reader)?);
        }
        Ok(array)
    }

    fn skip_array(&self, type_: &ArrayPropDescriptor, reader: &mut BitReader) -> std::io::Result<()> {
        let len_bits = num_bits(type_.num_elements);
        let len = reader.read::<u32>(len_bits)?;
        for _ in 0..len {
            self.skip_scalar(&type_.elem_type, reader)?;
        }
        Ok(())
    }

    fn decode_int(&self, desc: &IntPropDescriptor, reader: &mut BitReader) -> io::Result<i32> {
        let val = if desc.varint {
            if desc.unsigned {
                reader.read_varint32()? as i32
            } else {
                reader.read_signed_varint32()?
            }
        } else if desc.unsigned {
            reader.read::<u32>(desc.num_bits)? as i32
        } else {
            reader.read::<i32>(desc.num_bits)?
        };
        Ok(val)
    }

    fn skip_int(&self, desc: &IntPropDescriptor, reader: &mut BitReader) -> io::Result<()> {
        if desc.varint {
            if desc.unsigned {
                reader.read_varint32()?;
            } else {
                reader.read_signed_varint32()?;
            }
            Ok(())
        } else {
            reader.skip(desc.num_bits)
        }
    }

    fn decode_float(&self, f: &FloatPropDescriptor, reader: &mut BitReader) -> io::Result<f32> {
        let val = match f.special {
            FloatEncoding::Scaled => {
                let int = reader.read::<u32>(f.num_bits)? as f32;
                let unscaled = int / ((1 << f.num_bits) - 1) as f32;
                f.low_value + (f.high_value - f.low_value) * unscaled
            }
            FloatEncoding::Coord => reader.read_coord()?,
            FloatEncoding::CoordMP => reader.read_coord_mp(CoordType::None)?,
            FloatEncoding::CoordMPLowPrecision => reader.read_coord_mp(CoordType::LowPrecision)?,
            FloatEncoding::CoordMPIntegral => reader.read_coord_mp(CoordType::Integral)?,
            FloatEncoding::NoScale => f32::from_bits(reader.read::<u32>(32)?),
            FloatEncoding::Normal => reader.read_normal()?,
            FloatEncoding::CellCoord => reader.read_cell_coord(f.num_bits, CoordType::None)?,
            FloatEncoding::CellCoordLowPrecision => {
                reader.read_cell_coord(f.num_bits, CoordType::LowPrecision)?
            }
            FloatEncoding::CellCoordIntegral => {
                reader.read_cell_coord(f.num_bits, CoordType::Integral)?
            }
        };
        Ok(val)
    }

    fn skip_float(&self, f: &FloatPropDescriptor, reader: &mut BitReader) -> io::Result<()> {
        match f.special {
            FloatEncoding::Scaled => reader.skip(f.num_bits),
            FloatEncoding::Coord => reader.skip_coord(),
            FloatEncoding::CoordMP => reader.skip_coord_mp(CoordType::None),
            FloatEncoding::CoordMPLowPrecision => reader.skip_coord_mp(CoordType::LowPrecision),
            FloatEncoding::CoordMPIntegral => reader.skip_coord_mp(CoordType::Integral),
            FloatEncoding::NoScale => reader.skip(32),
            FloatEncoding::Normal => reader.skip_normal(),
            FloatEncoding::CellCoord => reader.skip_cell_coord(f.num_bits, CoordType::None),
            FloatEncoding::CellCoordLowPrecision => {
                reader.skip_cell_coord(f.num_bits, CoordType::LowPrecision)
            }
            FloatEncoding::CellCoordIntegral => {
                reader.skip_cell_coord(f.num_bits, CoordType::Integral)
            }
        }
    }

    fn decode_vector_xy(
        &self,
        f: &FloatPropDescriptor,
        reader: &mut BitReader,
    ) -> io::Result<Vector> {
        let x = self.decode_float(f, reader)?;
        let y = self.decode_float(f, reader)?;
        let z = 0_f32;
        Ok(Vector { x, y, z })
    }

    fn skip_vector_xy(&self, f: &FloatPropDescriptor, reader: &mut BitReader) -> io::Result<()> {
        self.skip_float(f, reader)?;
        self.skip_float(f, reader)
    }

    fn decode_vector(
        &self,
        f: &FloatPropDescriptor,
        unit: bool,
        reader: &mut BitReader,
    ) -> io::Result<Vector> {
        let x = self.decode_float(f, reader)?;
        let y = self.decode_float(f, reader)?;
        let z = if unit {
            let x2y2 = x * x + y * y;
            let abs = if x2y2 < 1_f32 {
                (1_f32 - x2y2).sqrt()
            } else {
                0_f32
            };
            if reader.read_bit()? {
                -abs
            } else {
                abs
            }
        } else {
            self.decode_float(f, reader)?
        };
        Ok(Vector { x, y, z })
    }

    fn skip_vector(
        &self,
        f: &FloatPropDescriptor,
        unit: bool,
        reader: &mut BitReader,
    ) -> io::Result<()> {
        self.skip_float(f, reader)?;
        self.skip_float(f, reader)?;
        if unit {
            reader.skip(1)
        } else {
            self.skip_float(f, reader)
        }
    }

    fn decode_string(&self, reader: &mut BitReader) -> std::io::Result<String> {
        let len = reader.read::<u32>(9)? as usize;
        let mut buf = vec![0; len];
        reader.read_bytes(buf.as_mut_slice())?;
        let val = String::from_utf8(buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(val)
    }

    fn skip_string(&self, reader: &mut BitReader) -> std::io::Result<()> {
        let len = reader.read::<u32>(9)?;
        reader.skip(len * 8)?;
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DecodeError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Protobuf(#[from] FromUtf8Error),
}

#[derive(Debug)]
enum ScalarPropDescriptor {
    Int(IntPropDescriptor),
    Float(FloatPropDescriptor),
    Vector(FloatPropDescriptor, bool),
    VectorXY(FloatPropDescriptor),
    String,
    Int64,
}

impl ScalarPropDescriptor {
    fn try_new(sendprop: &Sendprop_t) -> Result<Self> {
        Ok(match sendprop.type_() {
            DPT_INT => Self::Int(IntPropDescriptor::from(sendprop)),
            DPT_FLOAT => Self::Float(FloatPropDescriptor::from(sendprop)),
            DPT_VECTOR => Self::Vector(
                FloatPropDescriptor::from(sendprop),
                sendprop.flags() & SPROP_NORMAL != 0,
            ),
            DPT_VECTOR_XY => Self::VectorXY(FloatPropDescriptor::from(sendprop)),
            DPT_STRING => Self::String,
            DPT_INT64 => Self::Int64,
            _ => Err(Error::ServerClass(
                "invalid scalar sendprop type".to_string(),
            ))?,
        })
    }
}

#[derive(Debug)]
enum PropDescriptorType {
    Scalar(ScalarPropDescriptor),
    Array(ArrayPropDescriptor),
}

pub(crate) struct ServerClassesParser<'a> {
    send_tables: HashMap<&'a str, &'a CSVCMsg_SendTable>,
}

impl<'a> ServerClassesParser<'a> {
    pub(crate) fn parse(
        send_tables: &'a [CSVCMsg_SendTable],
        server_classes: &'a [csvcmsg_class_info::Class_t],
    ) -> Result<Vec<ServerClass>> {
        let send_tables: HashMap<&str, &CSVCMsg_SendTable> = send_tables
            .iter()
            .map(|st| (st.net_table_name(), st))
            .collect();
        Self { send_tables }.parse_server_classes(server_classes)
    }

    fn lookup_data_table(&self, name: &str) -> Result<&'a CSVCMsg_SendTable> {
        self.send_tables
            .get(name)
            .copied()
            .ok_or_else(|| Error::ServerClass(format!("table name {name} not found")))
    }

    fn parse_server_classes(
        &self,
        server_classes: &'a [csvcmsg_class_info::Class_t],
    ) -> Result<Vec<ServerClass>> {
        for (i, sc) in server_classes.iter().enumerate() {
            if i != sc.class_id() as usize {
                return Err(Error::ServerClass(
                    "server class id not sequential".to_string(),
                ));
            }
        }
        server_classes
            .iter()
            .map(|c| self.parse_server_class(c))
            .collect()
    }

    /// Collects all properties sent by the server for entities of this class.
    ///
    /// Each data table records the properties sent by the server for data members defined
    /// within that class. The data table will also link to data tables for base classes.
    /// We have to collect properties from the data table and any other linked data tables,
    /// except for excluded properties. See [Valve doc].
    ///
    /// [Valve doc]: https://developer.valvesoftware.com/wiki/Networking_Entities#Network_Data_Tables
    fn parse_server_class(&self, class_ref: &csvcmsg_class_info::Class_t) -> Result<ServerClass> {
        let table = self.lookup_data_table(class_ref.data_table_name())?;
        let mut excludes = Vec::new();
        self.gather_excludes(table, &mut excludes)?;
        let mut prio_props = Vec::new();
        self.gather_class_props(table, &excludes, &mut prio_props)?;
        // We have to use this specific sorting algorithm because the order of
        // the props with the same priority matters.
        selection_sort_by_prio(&mut prio_props);
        let name = class_ref.class_name();
        let props = prio_props.into_iter().map(|i| i.1).collect();
        Ok(ServerClass::new(name, props))
    }

    fn gather_excludes(
        &self,
        table: &'a CSVCMsg_SendTable,
        excludes: &mut Vec<(&'a str, &'a str)>,
    ) -> Result<()> {
        for prop in &table.props {
            if prop.flags() & SPROP_EXCLUDE != 0 {
                excludes.push((prop.dt_name(), prop.var_name()))
            }
            if prop.type_() == DPT_DATA_TABLE {
                self.gather_excludes(self.lookup_data_table(prop.dt_name())?, excludes)?
            }
        }
        Ok(())
    }

    fn gather_class_props(
        &self,
        table: &CSVCMsg_SendTable,
        excludes: &Vec<(&str, &str)>,
        result: &mut Vec<(i32, PropDescriptor)>,
    ) -> Result<()> {
        let mut tmp = Vec::new();
        self.gather_props(table, excludes, &mut tmp, result)?;
        result.append(&mut tmp);
        Ok(())
    }

    fn gather_props(
        &self,
        table: &CSVCMsg_SendTable,
        excludes: &Vec<(&'a str, &'a str)>,
        current: &mut Vec<(i32, PropDescriptor)>,
        result: &mut Vec<(i32, PropDescriptor)>,
    ) -> Result<()> {
        let mut array_elem = None;
        for sendprop in &table.props {
            // sendprop.dt_name() is only set for data tables and exclude props.
            // To check if an exclude applies, we need to use the current table
            // name.
            let qualified_name = (table.net_table_name(), sendprop.var_name());
            if sendprop.flags() & SPROP_EXCLUDE != 0 || excludes.contains(&qualified_name) {
                continue;
            }
            if sendprop.type_() == DPT_DATA_TABLE {
                let table = self.lookup_data_table(sendprop.dt_name())?;
                if sendprop.flags() & SPROP_COLLAPSIBLE != 0 {
                    self.gather_props(table, excludes, current, result)?;
                } else {
                    self.gather_class_props(table, excludes, result)?;
                }
            } else if sendprop.type_() == DPT_ARRAY {
                let array_elem = array_elem.ok_or_else(|| {
                    Error::ServerClass(format!(
                        "array sendprop without preceding element: {}.{}",
                        sendprop.dt_name(),
                        sendprop.var_name()
                    ))
                })?;
                let type_ =
                    PropDescriptorType::Array(ArrayPropDescriptor::try_new(sendprop, array_elem)?);
                let prio = prop_priority(sendprop);
                current.push((prio, PropDescriptor::new(sendprop.var_name(), type_)));
            } else if sendprop.flags() & SPROP_INSIDEARRAY != 0 {
                array_elem = Some(sendprop);
            } else {
                let prio = prop_priority(sendprop);
                current.push((
                    prio,
                    PropDescriptor::new(
                        sendprop.var_name(),
                        PropDescriptorType::Scalar(ScalarPropDescriptor::try_new(sendprop)?),
                    ),
                ));
            }
        }
        Ok(())
    }
}

/// Selection sort, taking advantage of the fact that the number of priorities is very small.
fn selection_sort_by_prio<T>(v: &mut [(i32, T)]) {
    let mut priorities = Vec::with_capacity(16);
    for e in v.iter() {
        if !priorities.contains(&e.0) {
            priorities.push(e.0);
        }
    }
    priorities.sort();
    let mut i = 0;
    for prio in priorities {
        while i < v.len() && v[i].0 == prio {
            i += 1;
        }
        for j in i + 1..v.len() {
            if v[j].0 == prio {
                if i != j {
                    v.swap(i, j);
                }
                i += 1;
            }
        }
    }
}

// Extract the priority used for sorting the sendprops within a server class.
fn prop_priority(sendprop: &Sendprop_t) -> i32 {
    if sendprop.flags() & SPROP_CHANGESOFTEN != 0 {
        sendprop.priority().min(64)
    } else {
        sendprop.priority()
    }
}

/// Unsigned integer.
const SPROP_UNSIGNED: i32 = 1 << 0;
/// The vector is a unit vector.
const SPROP_NORMAL: i32 = 1 << 5;
/// Exclude a property from base classes.
const SPROP_EXCLUDE: i32 = 1 << 6;
/// The property is inside an array.
const SPROP_INSIDEARRAY: i32 = 1 << 8;
/// Use var int encoded (google protobuf style).
const SPROP_VARINT: i32 = 1 << 19;
// Set automatically if it's a datatable with an offset of 0 that doesn't change the pointer (ie: for all automatically-chained base classes).
const SPROP_COLLAPSIBLE: i32 = 1 << 11;
// Sets the priority to min(prio, 64) for the purposes of sorting the properties.
const SPROP_CHANGESOFTEN: i32 = 1 << 18;

const DPT_INT: i32 = 0;
const DPT_FLOAT: i32 = 1;
const DPT_VECTOR: i32 = 2;
const DPT_VECTOR_XY: i32 = 3;
const DPT_STRING: i32 = 4;
const DPT_ARRAY: i32 = 5;
const DPT_DATA_TABLE: i32 = 6;
const DPT_INT64: i32 = 7;

#[cfg(test)]
mod tests {
    use super::{FloatEncoding, ServerClassesParser};
    use protobuf::text_format::parse_from_str;

    #[test]
    fn float_encoding() {
        // Coord* special types have higher priority than NoScale. Some demo files set both flags.
        for bit in [1, 12, 13, 14] {
            assert_ne!(
                FloatEncoding::from_flags((1 << bit) + (1 << 2)),
                FloatEncoding::NoScale
            );
        }
    }

    #[test]
    fn collapsible_order() {
        let st1 = parse_from_str(
            r#"net_table_name: "st1"
               props { type: 0 var_name: "1" }
               props { type: 6 flags: 2048 dt_name: "st2" }
               props { type: 6 dt_name: "st3" }"#,
        )
        .unwrap();
        let st2 = parse_from_str(
            r#"net_table_name: "st2"
               props { type: 0 var_name: "2" }"#,
        )
        .unwrap();
        let st3 = parse_from_str(
            r#"net_table_name: "st3"
               props { type: 0 var_name: "3" }"#,
        )
        .unwrap();
        let class = parse_from_str(r#"class_id: 0 data_table_name: "st1""#).unwrap();
        let send_tables = vec![st1, st2, st3];
        let server_classes = vec![class];
        let server_classes = ServerClassesParser::parse(&send_tables, &server_classes).unwrap();
        let order: Vec<&str> = server_classes
            .first()
            .unwrap()
            .props
            .iter()
            .map(|pd| pd.name.as_str())
            .collect();
        assert_eq!(order, vec!["3", "1", "2"]);
    }
}
