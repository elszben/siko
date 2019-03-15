use crate::location_info::item::LocationId;
use crate::syntax::expr::ExprId;
use crate::syntax::types::TypeSignatureId;

#[derive(Debug, Clone)]
pub enum FunctionBody {
    Expr(ExprId),
    Extern,
}

#[derive(Debug, Clone)]
pub struct FunctionType {
    pub name: String,
    pub type_args: Vec<String>,
    pub full_type_signature_id: TypeSignatureId,
    pub type_signature_id: TypeSignatureId,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub id: FunctionId,
    pub name: String,
    pub args: Vec<String>,
    pub body: FunctionBody,
    pub func_type: Option<FunctionType>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct FunctionId {
    pub id: usize,
}
