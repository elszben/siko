use crate::data::TypeDefId;
use crate::expr::ExprId;
use crate::types::Type;
use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct FunctionId {
    pub id: usize,
}

impl fmt::Display for FunctionId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "func#{}", self.id)
    }
}

impl From<usize> for FunctionId {
    fn from(id: usize) -> FunctionId {
        FunctionId { id: id }
    }
}

pub enum FunctionInfo {
    Normal(ExprId),
    Extern(String),
    VariantConstructor(TypeDefId, usize),
    RecordConstructor(TypeDefId),
}

pub struct Function {
    pub name: String,
    pub module: String,
    pub function_type: Type,
    pub arg_count: usize,
    pub info: FunctionInfo,
}
