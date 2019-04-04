use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeVariable {
    pub id: usize,
}

impl TypeVariable {
    pub fn new(id: usize) -> TypeVariable {
        TypeVariable { id: id }
    }
}

impl fmt::Display for TypeVariable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Tv{}", self.id)
    }
}
