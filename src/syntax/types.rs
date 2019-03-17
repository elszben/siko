use crate::syntax::item_path::ItemPath;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeSignatureId {
    pub id: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum TypeSignature {
    Nothing,
    Named(ItemPath, Vec<TypeSignatureId>),
    Tuple(Vec<TypeSignatureId>),
    Function(Vec<TypeSignatureId>),
    TypeArgument(ItemPath),
}
