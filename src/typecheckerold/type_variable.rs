#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeVariable {
    pub id: usize,
}

impl TypeVariable {
    pub fn new(id: usize) -> TypeVariable {
        TypeVariable { id: id }
    }
}
