use crate::ir::function::FunctionId;
use crate::location_info::item::LocationId;
use crate::syntax::data::AdtId as AstAdtId;
use crate::syntax::data::RecordId as AstRecordId;
use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeSignatureId {
    pub id: usize,
}

#[derive(Debug, Clone)]
pub enum TypeSignature {
    Bool,
    Int,
    String,
    Nothing,
    Tuple(Vec<TypeSignatureId>),
    Function(TypeSignatureId, TypeSignatureId),
    TypeArgument(usize, String),
    Named(TypeDefId, Vec<TypeSignatureId>),
    Variant(String, Vec<TypeSignatureId>),
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub type_signature: TypeSignature,
    pub location_id: LocationId,
}

impl TypeInfo {
    pub fn new(type_signature: TypeSignature, location_id: LocationId) -> TypeInfo {
        TypeInfo {
            type_signature: type_signature,
            location_id: location_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecordField {
    pub name: String,
    pub type_signature_id: TypeSignatureId,
}

#[derive(Debug, Clone)]
pub struct Record {
    pub name: String,
    pub ast_record_id: AstRecordId,
    pub id: TypeDefId,
    pub type_arg_count: usize,
    pub fields: Vec<RecordField>,
    pub constructor: FunctionId,
}

#[derive(Debug, Clone)]
pub struct VariantItem {
    pub type_signature_id: TypeSignatureId,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub items: Vec<VariantItem>,
    pub type_signature_id: TypeSignatureId,
    pub constructor: FunctionId,
}

#[derive(Debug, Clone)]
pub struct Adt {
    pub name: String,
    pub ast_adt_id: AstAdtId,
    pub id: TypeDefId,
    pub type_arg_count: usize,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone)]
pub enum TypeDef {
    Record(Record),
    Adt(Adt),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeDefId {
    pub id: usize,
}

impl fmt::Display for TypeDefId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TypeDefId({})", self.id)
    }
}
