use crate::location_info::item::LocationId;
use crate::syntax::types::TypeSignatureId;

#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub members: Vec<ClassMember>,
    pub location_id: LocationId
}

#[derive(Debug, Clone)]
pub struct ClassMember {
    pub name: String,
    pub type_signature_id: TypeSignatureId,
    pub location_id: LocationId
}