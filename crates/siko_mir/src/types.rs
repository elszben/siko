use crate::data::TypeDefId;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Type {
    Tuple(Vec<Type>),
    Named(String, TypeDefId, Vec<Type>),
    Function(Box<Type>, Box<Type>),
}
