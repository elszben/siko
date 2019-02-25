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

impl fmt::Display for NamedFunctionInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.module, self.name)
    }
}

#[derive(Debug, Clone)]
pub enum FunctionInfo {
    Lambda(ExprId),
    NamedFunction(NamedFunctionInfo),
}

impl FunctionInfo {
    pub fn typed(&self) -> bool {
        match self {
            FunctionInfo::Lambda(_) => false,
            FunctionInfo::NamedFunction(i) => i.type_signature.is_some()
        }
    }
}

impl fmt::Display for FunctionInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FunctionInfo::Lambda(e) => write!(f, "lambda{}", e),
            FunctionInfo::NamedFunction(i) => write!(f, "{}", i),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub id: FunctionId,
    pub info: FunctionInfo,
}
