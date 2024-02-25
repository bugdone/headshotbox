use std::rc::Rc;

#[derive(Clone)]
pub(super) enum PathNameItem {
    Field(Rc<str>),
    Index(usize),
}

#[derive(Clone)]
pub struct PathName {
    pub(super) items: Vec<PathNameItem>,
}

impl PathName {
    pub(super) fn push_field(mut self, field: Rc<str>) -> Self {
        self.items.push(PathNameItem::Field(field));
        self
    }

    pub(super) fn push_index(mut self, index: usize) -> Self {
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
