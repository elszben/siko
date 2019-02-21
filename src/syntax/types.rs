#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeSignatureId {
    pub id: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TypeSignature {
    Nothing,
    Named(String),
    Tuple(Vec<TypeSignatureId>),
    Function(Vec<TypeSignatureId>),
}
