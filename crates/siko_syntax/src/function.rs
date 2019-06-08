use crate::class::Constraint;
use crate::expr::ExprId;
use crate::types::TypeSignatureId;
use siko_location_info::item::LocationId;

#[derive(Debug, Clone)]
pub enum FunctionBody {
    Expr(ExprId),
    Extern,
}

#[derive(Debug, Clone)]
pub struct FunctionType {
    pub name: String,
    pub type_args: Vec<(String, LocationId)>,
    pub constraints: Vec<Constraint>,
    pub full_type_signature_id: TypeSignatureId,
    pub type_signature_id: TypeSignatureId,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub id: FunctionId,
    pub name: String,
    pub args: Vec<(String, LocationId)>,
    pub body: FunctionBody,
    pub func_type: Option<FunctionType>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct FunctionId {
    pub id: usize,
}
