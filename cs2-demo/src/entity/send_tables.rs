use core::fmt;
use std::collections::HashMap;
use std::rc::Rc;

use protobuf::CodedInputStream;

use crate::proto::demo::CDemoSendTables;
use crate::proto::netmessages::CSVCMsg_FlattenedSerializer;
use crate::{Error, Result};

use super::decoder::{decode_float32, decode_qangle, decode_vector, Decoder};

#[derive(Debug, Default)]
pub struct SendTables {
    pub(super) serializers: Vec<Rc<Serializer>>,
}

#[derive(Debug)]
pub struct Serializer {
    pub(super) name: String,
    pub(super) fields: Vec<Field>,
}

impl std::fmt::Display for Serializer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "class {}", self.name)?;
        for field in &self.fields {
            writeln!(f, "  {field:?}")?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct ValueField {
    decoder: Decoder,
    var_name: Rc<str>,
    var_type: Rc<str>,
}
#[derive(Clone)]
pub struct ArrayField {
    decoder: Decoder,
    pub(super) size: u16,
    pub(super) element: Box<Field>,
}
#[derive(Clone)]
pub struct VectorField {
    decoder: Decoder,
    pub(super) element: Box<Field>,
}
#[derive(Clone)]
pub struct ObjectField {
    decoder: Decoder,
    var_name: Rc<str>,
    var_type: Rc<str>,
    pub(super) serializer: Rc<Serializer>,
}

#[derive(Debug, Clone)]
pub enum Field {
    Value(ValueField),
    Object(ObjectField),
    Array(ArrayField),
    Vector(VectorField),
}

impl Field {
    pub fn name(&self) -> Rc<str> {
        match self {
            Field::Value(v) => Rc::clone(&v.var_name),
            Field::Object(v) => Rc::clone(&v.var_name),
            Field::Array(v) => v.element.name(),
            Field::Vector(v) => v.element.name(),
        }
    }

    pub fn ctype(&self) -> &str {
        match self {
            Field::Value(v) => v.var_type.as_ref(),
            Field::Object(v) => v.var_type.as_ref(),
            Field::Array(v) => v.element.ctype(),
            Field::Vector(v) => v.element.ctype(),
        }
    }

    pub(super) fn decoder(&self) -> &Decoder {
        match self {
            Field::Value(v) => &v.decoder,
            Field::Object(v) => &v.decoder,
            Field::Array(v) => &v.decoder,
            Field::Vector(v) => &v.decoder,
        }
    }
}

impl std::fmt::Debug for ValueField {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("Value")
            .field("var_name", &self.var_name)
            .field("var_type", &self.var_type)
            .finish()
    }
}
impl std::fmt::Debug for ArrayField {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("Array")
            .field("size", &self.size)
            .field("element", &self.element)
            .finish()
    }
}
impl std::fmt::Debug for VectorField {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("Vector")
            .field("element", &self.element)
            .finish()
    }
}
impl std::fmt::Debug for ObjectField {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        f.debug_struct("Object")
            .field("var_name", &self.var_name)
            .field("var_type", &self.var_type)
            .field("serializer", &self.serializer)
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
                let name = s.serializer_name_sym.unwrap();
                let version = s.serializer_version.unwrap();
                ((name, version), i as i32)
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
}

impl<'a> SendTableBuilder<'a> {
    fn new(data: &'a SentTableData) -> Self {
        let serializers = vec![None; data.fs.serializers.len()];
        Self { data, serializers }
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

    fn lookup_serializer(
        &mut self,
        name_sym: Option<i32>,
        version: Option<i32>,
    ) -> Option<Rc<Serializer>> {
        match (name_sym, version) {
            (Some(name), Some(version)) => {
                let si = self.data.by_name_ver.get(&(name, version)).cloned();
                si.map(|si| self.serializer(si))
            }
            _ => None,
        }
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
        let serializer = match (fsf.field_serializer_name_sym, fsf.field_serializer_version) {
            (Some(name), Some(version)) => {
                let si = self.data.by_name_ver.get(&(name, version)).cloned();
                si.map(|si| self.serializer(si))
            }
            _ => None,
        };
        let polymorphic_types = fsf
            .polymorphic_types
            .iter()
            .map(|t| {
                self.lookup_serializer(
                    t.polymorphic_field_serializer_name_sym,
                    t.polymorphic_field_serializer_version,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let ctype = CType::parse(var_type.as_ref());
        let field = if let Some(serializer) = serializer {
            let decoder = if !polymorphic_types.is_empty() {
                Decoder::Polymorphic(serializer.clone())
            } else {
                Decoder::Object(serializer.clone())
            };
            Field::Object(ObjectField {
                var_name: Rc::clone(&var_name),
                var_type: Rc::clone(&var_type),
                decoder,
                serializer,
            })
        } else {
            let decoder = make_decoder(
                ctype.0,
                encoder,
                fsf.bit_count(),
                fsf.low_value.unwrap_or(0f32),
                fsf.high_value.unwrap_or(1f32),
                fsf.encode_flags(),
            );
            Field::Value(ValueField {
                var_name: Rc::clone(&var_name),
                var_type: Rc::clone(&var_type),
                decoder,
            })
        };
        match ctype.2 {
            ArraySize::None => field,
            ArraySize::Fixed(size) => match ctype.0 {
                "char" => Field::Value(ValueField {
                    var_name,
                    var_type,
                    decoder: Decoder::String,
                }),
                _ => Field::Array(ArrayField {
                    size,
                    decoder: Decoder::None,
                    element: Box::new(field),
                }),
            },
            ArraySize::Variable => Field::Vector(VectorField {
                decoder: Decoder::U32,
                element: Box::new(field),
            }),
        }
    }
}

fn make_decoder(
    base_type: &str,
    encoder: Option<&str>,
    bit_count: i32,
    low_value: f32,
    high_value: f32,
    encode_flags: i32,
) -> Decoder {
    match base_type {
        "int8"
        | "int16"
        | "int32"
        | "BeamType_t"
        | "CEntityIndex"
        | "EntityDisolveType_t"
        | "HSequence" => Decoder::I32,
        "uint64" | "CStrongHandle" => match encoder {
            Some("fixed64") => Decoder::Fixed64,
            Some(s) => todo!("{}", s),
            None => Decoder::U64,
        },
        "uint8"
        | "uint16"
        | "uint32"
        | "attributeprovidertypes_t"
        | "loadout_slot_t"
        | "tablet_skin_state_t"
        | "AnimLoopMode_t"
        | "AttachmentHandle_t"
        | "BeamClipStyle_t"
        | "CEntityHandle"
        | "CHandle"
        | "CGameSceneNodeHandle"
        | "Color"
        | "CPlayerSlot"
        | "CSWeaponMode"
        | "CSWeaponState_t"
        | "CSPlayerBlockingUseAction_t"
        | "CSPlayerState"
        | "CUtlStringToken"
        | "DoorState_t"
        | "EGrenadeThrowState"
        | "EKillTypes_t"
        | "ESurvivalGameRuleDecision_t"
        | "ESurvivalSpawnTileState"
        | "FixAngleSet_t"
        | "GameTick_t"
        | "MedalRank_t"
        | "MoveType_t"
        | "MoveCollide_t"
        | "PlayerConnectedState"
        | "PlayerAnimEvent_t"
        | "PointWorldTextJustifyHorizontal_t"
        | "PointWorldTextJustifyVertical_t"
        | "PointWorldTextReorientMode_t"
        | "QuestProgress::Reason"
        | "RelativeDamagedDirection_t"
        | "RenderFx_t"
        | "RenderMode_t"
        | "ShardSolid_t"
        | "ShatterPanelMode"
        | "SolidType_t"
        | "SpawnStage_t"
        | "SurroundingBoundsType_t"
        | "ValueRemapperRatchetType_t"
        | "ValueRemapperHapticsType_t"
        | "ValueRemapperMomentumType_t"
        | "ValueRemapperInputType_t"
        | "ValueRemapperOutputType_t"
        | "WeaponAttackType_t"
        | "WeaponState_t"
        | "WorldGroupId_t" => Decoder::U32,
        "bool" => Decoder::Bool,
        "float32" | "CNetworkedQuantizedFloat" => {
            decode_float32(encoder, bit_count, low_value, high_value, encode_flags)
        }
        "char" | "CUtlString" | "CUtlSymbolLarge" => Decoder::String,
        "GameTime_t" => Decoder::NoScale,
        "QAngle" => decode_qangle(encoder, bit_count),
        "Vector2D" => decode_vector(2, encoder, bit_count, low_value, high_value, encode_flags),
        "Vector" => decode_vector(3, encoder, bit_count, low_value, high_value, encode_flags),
        "Vector4D" | "Quaternion" => {
            decode_vector(4, encoder, bit_count, low_value, high_value, encode_flags)
        }
        "CTransform" => decode_vector(6, encoder, bit_count, low_value, high_value, encode_flags),
        _ => todo!("{}", base_type),
    }
}

#[derive(Clone, Debug)]
enum ArraySize {
    None,
    Fixed(u16),
    Variable,
}

impl fmt::Display for ArraySize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArraySize::None => Ok(()),
            ArraySize::Fixed(s) => write!(f, "[{s}]"),
            ArraySize::Variable => write!(f, "[]"),
        }
    }
}

struct CType<'a>(&'a str, Option<Box<CType<'a>>>, ArraySize);

impl<'a> CType<'a> {
    fn parse(s: &'a str) -> Self {
        let (s, array) = match s.split_once('[') {
            Some((s, a)) => (
                s,
                ArraySize::Fixed(a[..a.len() - 1].parse::<u16>().unwrap()),
            ),
            None => (s, ArraySize::None),
        };
        let (base, param) = match s.find('<') {
            Some(open) => {
                let close = s.rfind('>').unwrap();
                (
                    &s[..open],
                    Some(Box::new(CType::parse(&s[open + 2..close - 1]))),
                )
            }
            None => (s, None),
        };
        match base {
            "CUtlVector" | "CNetworkUtlVectorBase" | "CUtlVectorEmbeddedNetworkVar" => {
                let base = param.unwrap();
                CType(base.0, base.1, ArraySize::Variable)
            }
            _ => CType(base, param, array),
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
        let data = SentTableData::new(fs);
        for f in data.fs.fields {
            let mut f = f.clone();
            f.clear_send_node_sym();
            let resolve =
                |sym: &mut Option<i32>| data.symbols[sym.take().unwrap() as usize].as_ref();
            let var_name = resolve(&mut f.var_name_sym);
            let var_type = resolve(&mut f.var_type_sym);
            print!("{}: {} ", var_name, var_type);
            if f.var_encoder_sym.is_some() {
                print!("encoder: {} ", resolve(&mut f.var_encoder_sym));
            }
            if f.field_serializer_name_sym.is_some() {
                print!("serializer: {} ", resolve(&mut f.field_serializer_name_sym));
            }
            if !f.polymorphic_types.is_empty() {
                for pt in f.polymorphic_types.iter_mut() {
                    print!(
                        "poly: {} ",
                        resolve(&mut pt.polymorphic_field_serializer_name_sym)
                    )
                }
                f.polymorphic_types.clear();
            }
            println!("{f}");
        }
    }
}
