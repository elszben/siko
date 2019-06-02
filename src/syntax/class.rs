use crate::location_info::item::LocationId;
use crate::syntax::function::FunctionId;
use crate::syntax::function::FunctionType;
use crate::syntax::types::TypeSignatureId;

#[derive(Debug, Clone)]
pub struct Class {
    pub id: ClassId,
    pub name: String,
    pub arg: String,
    pub constraints: Vec<Constraint>,
    pub members: Vec<ClassMemberId>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub id: InstanceId,
    pub name: String,
    pub type_signature_id: TypeSignatureId,
    pub constraints: Vec<Constraint>,
    pub members: Vec<InstanceMember>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub struct ClassMember {
    pub id: ClassMemberId,
    pub type_signature: FunctionType,
    pub function: Option<FunctionId>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub struct InstanceMember {
    pub function: FunctionId,
}

#[derive(Debug, Clone)]
pub struct Constraint {
    pub class_name: String,
    pub arg: String,
    pub location_id: LocationId,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ClassId {
    pub id: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ClassMemberId {
    pub id: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct InstanceId {
    pub id: usize,
}
