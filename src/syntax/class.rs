use crate::location_info::item::LocationId;
use crate::syntax::function::FunctionId;
use crate::syntax::function::FunctionType;

#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub argument: String,
    pub members: Vec<ClassMember>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub struct ClassMember {
    pub type_signature: FunctionType,
    pub function: Option<FunctionId>,
}