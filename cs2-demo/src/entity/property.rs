pub type Properties = Vec<Option<Property>>;

#[derive(Debug, Clone)]
pub enum Property {
    Bool(bool),
    I32(i32),
    U32(u32),
    F32(f32),
    Str(Box<str>),
    Object(Properties),
}
