use crate::function::FunctionId;
use crate::function::FunctionTypeId;
use crate::types::TypeSignatureId;
use siko_location_info::item::LocationId;

#[derive(Debug, Clone)]
pub struct Class {
    pub id: ClassId,
    pub name: String,
    pub arg: String,
    pub constraints: Vec<Constraint>,
    pub member_functions: Vec<FunctionId>,
    pub member_function_types: Vec<FunctionTypeId>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub id: InstanceId,
    pub class_name: String,
    pub type_signature_id: TypeSignatureId,
    pub constraints: Vec<Constraint>,
    pub member_functions: Vec<FunctionId>,
    pub member_function_types: Vec<FunctionTypeId>,
    pub location_id: LocationId,
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

impl From<usize> for ClassId {
    fn from(id: usize) -> ClassId {
        ClassId { id: id }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct InstanceId {
    pub id: usize,
}

impl From<usize> for InstanceId {
    fn from(id: usize) -> InstanceId {
        InstanceId { id: id }
    }
}
