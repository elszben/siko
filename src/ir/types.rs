use crate::syntax::data::AdtId as AstAdtId;
use crate::syntax::data::RecordId as AstRecordId;
use crate::syntax::types::TypeSignatureId as AstTypeSignatureId;

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
    Function(Vec<TypeSignatureId>),
    TypeArgument(usize),
    Named(TypeDefId, Vec<TypeSignatureId>),
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub type_signature: TypeSignature,
    pub ast_type_id: AstTypeSignatureId,
}

impl TypeInfo {
    pub fn new(type_signature: TypeSignature, ast_type_id: AstTypeSignatureId) -> TypeInfo {
        TypeInfo {
            type_signature: type_signature,
            ast_type_id: ast_type_id,
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
}

#[derive(Debug, Clone)]
pub struct VariantItem {
    pub type_signature_id: TypeSignatureId,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub type_signature_id: TypeSignatureId,
    pub items: Vec<VariantItem>,
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
