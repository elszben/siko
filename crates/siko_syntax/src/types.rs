#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeSignatureId {
    pub id: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TypeSignature {
    Nothing,
    TypeArg(String),
    Named(String, Vec<TypeSignatureId>),
    Variant(String, Vec<TypeSignatureId>),
    Tuple(Vec<TypeSignatureId>),
    Function(TypeSignatureId, TypeSignatureId),
    Wildcard,
}