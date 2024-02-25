use std::{fmt, rc::Rc};

use super::path_name::PathNameItem;
use super::send_tables::{Field, Serializer};
use super::{Entity, PathName};

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
    Array(Vec<Option<Property>>),
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

#[derive(Debug)]
pub struct TreeEntity {
    object: Object,
    serializer: Rc<Serializer>,
}

impl TreeEntity {
    pub fn factory(serializer: Rc<Serializer>) -> Box<dyn Entity> {
        Box::new(Self {
            object: Object::new(&serializer),
            serializer,
        })
    }
}

impl Entity for TreeEntity {
    fn serializer(&self) -> &Rc<Serializer> {
        &self.serializer
    }

    fn get_property(&self, fp: &[i32]) -> (Option<&Property>, &Field, PathName) {
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

    fn set_property(&mut self, fp: &[i32], value: Option<Property>) {
        let prop = &mut self.object.properties[fp[0] as usize];
        let field = &self.serializer.fields[fp[0] as usize];
        let (prop, field) =
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
                    (p, Field::Array(f)) if p.is_none() => {
                        *p = Some(Property::Array(vec![None; f.size as usize]));
                        match p {
                            Some(Property::Array(a)) => (&mut a[i as usize], &f.element.as_ref()),
                            _ => unreachable!(),
                        }
                    }
                    (Some(Property::Array(a)), Field::Vector(f)) => {
                        if a.len() <= i as usize {
                            a.resize(i as usize + 1, Default::default());
                        }
                        (&mut a[i as usize], f.element.as_ref())
                    }
                    (Some(p), f) => unreachable!("{p:?} {f:?}"),
                    (None, f) => unreachable!("{f:?}"),
                });
        match field {
            Field::Value(_) => *prop = value,
            Field::Object(_) => *prop = value,
            Field::Array(_) => *prop = value,
            Field::Vector(v) => {
                let size = match value {
                    Some(Property::U32(size)) => size,
                    _ => unreachable!(),
                };
                let init = match v.element.as_ref() {
                    Field::Value(_) => None,
                    Field::Object(o) => Some(Property::Object(Object::new(&o.serializer))),
                    Field::Array(_) | Field::Vector(_) => unimplemented!(),
                };
                let vec = vec![init.clone(); size as usize];
                *prop = Some(Property::Array(vec));
            }
        };
    }
}

impl fmt::Display for TreeEntity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn dfs(
            f: &mut fmt::Formatter<'_>,
            path: PathName,
            prop: &Property,
            field: &Field,
        ) -> fmt::Result {
            let path = path.push_field(field.name());
            match (prop, field) {
                (Property::Object(o), Field::Object(fo)) => {
                    print_object(f, path, o, &fo.serializer)?
                }
                (Property::Array(a), Field::Array(fa)) => {
                    for (i, e) in a.iter().enumerate() {
                        let path = path.clone().push_index(i);
                        dfs(f, path, e.as_ref().unwrap(), &fa.element)?;
                    }
                }
                (Property::Array(a), Field::Vector(fv)) => {
                    for (i, e) in a.iter().enumerate() {
                        let path = path.clone().push_index(i);
                        dfs(f, path, e.as_ref().unwrap(), &fv.element)?;
                    }
                }
                _ => writeln!(f, "{} = {}", path, prop)?,
            }
            Ok(())
        }

        fn print_object(
            f: &mut fmt::Formatter<'_>,
            path: PathName,
            object: &Object,
            serializer: &Serializer,
        ) -> fmt::Result {
            for (i, e) in object.properties.iter().enumerate() {
                let field = &serializer.fields[i];
                if let Some(prop) = e {
                    dfs(f, path.clone(), prop, field)?;
                }
            }
            Ok(())
        }

        let path = PathName {
            items: vec![PathNameItem::Field(Rc::from(self.serializer.name.as_str()))],
        };
        print_object(f, path, &self.object, &self.serializer)
    }
}
