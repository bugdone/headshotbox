use std::rc::Rc;

use super::send_tables::Serializer;

#[derive(Debug, Clone)]
pub struct Object {
    pub(super) properties: Box<[Option<Property>]>,
}

impl Object {
    pub(super) fn new(serializer: &Rc<Serializer>) -> Self {
        let len = serializer.fields.len();
        Self {
            properties: vec![None; len].into_boxed_slice(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Property {
    Bool(bool),
    I32(i32),
    U32(u32),
    U64(u64),
    F32(f32),
    Str(Box<str>),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Vec6([f32; 6]),
    Object(Object),
    Array(Box<[Option<Property>]>),
}

impl std::fmt::Display for Property {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Property::Bool(p) => write!(f, "{p}"),
            Property::I32(p) => write!(f, "{p}"),
            Property::U32(p) => write!(f, "{p}"),
            Property::U64(p) => write!(f, "{p}"),
            Property::F32(p) => write!(f, "{p}"),
            Property::Str(p) => write!(f, "{p}"),
            Property::Vec2(p) => write!(f, "{p:?}"),
            Property::Vec3(p) => write!(f, "{p:?}"),
            Property::Vec4(p) => write!(f, "{p:?}"),
            Property::Vec6(p) => write!(f, "{p:?}"),
            Property::Object(p) => write!(f, "{p:?}"),
            Property::Array(p) => write!(f, "{}", p.len()),
        }
    }
}
