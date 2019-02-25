use crate::ir::expr::ExprId;
use crate::ir::types::TypeSignatureId;
use crate::syntax::function::FunctionId as AstFunctionId;
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

#[derive(Debug, Clone)]
pub struct NamedFunctionInfo {
    pub body: Option<ExprId>,
    pub module: String,
    pub name: String,
    pub type_signature: Option<TypeSignatureId>,
    pub ast_function_id: AstFunctionId,
}

#[derive(Debug, Clone)]
pub enum FunctionInfo {
    Lambda(ExprId),
    NamedFunction(NamedFunctionInfo),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub id: FunctionId,
    pub info: FunctionInfo,
}
