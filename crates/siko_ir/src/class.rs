use crate::function::FunctionId;
use crate::types::TypeSignatureId;
use siko_location_info::item::LocationId;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Class {
    pub id: ClassId,
    pub name: String,
    pub members: Vec<ClassMemberId>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ClassId {
    pub id: usize,
}

impl fmt::Display for ClassId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.id)
    }
}

impl From<usize> for ClassId {
    fn from(id: usize) -> ClassId {
        ClassId { id: id }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ClassMemberId {
    pub id: usize,
}

impl fmt::Display for ClassMemberId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.id)
    }
}

impl From<usize> for ClassMemberId {
    fn from(id: usize) -> ClassMemberId {
        ClassMemberId { id: id }
    }
}

#[derive(Debug, Clone)]
pub struct ClassMember {
    pub id: ClassMemberId,
    pub class_id: ClassId,
    pub name: String,
    pub class_type_signature: TypeSignatureId,
    pub type_signature: TypeSignatureId,
    pub default_implementation: Option<FunctionId>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct InstanceId {
    pub id: usize,
}

impl fmt::Display for InstanceId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.id)
    }
}

impl From<usize> for InstanceId {
    fn from(id: usize) -> InstanceId {
        InstanceId { id: id }
    }
}

#[derive(Debug, Clone)]
pub struct Instance {
    pub id: InstanceId,
    pub class_id: ClassId,
    pub type_signature: TypeSignatureId,
    pub members: Vec<InstanceMember>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub struct InstanceMember {
    pub class_member_id: ClassMemberId,
    pub function_id: FunctionId,
}
