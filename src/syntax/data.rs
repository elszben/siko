use crate::location_info::item::LocationId;
use crate::syntax::types::TypeSignatureId;

pub enum Data {
    Adt(Adt),
    Record(Record),
}

#[derive(Debug, Clone)]
pub struct Adt {
    pub name: String,
    pub id: AdtId,
    pub type_args: Vec<String>,
    pub variants: Vec<VariantId>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub id: VariantId,
    pub items: Vec<TypeSignatureId>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct AdtId {
    pub id: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct VariantId {
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct Record {
    pub data_name: String,
    pub name: String,
    pub id: RecordId,
    pub fields: Vec<RecordField>,
    pub location_id: LocationId,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct RecordId {
    pub id: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct RecordFieldId {
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct RecordField {
    pub name: String,
    pub id: RecordFieldId,
    pub type_signature_id: TypeSignatureId,
    pub location_id: LocationId,
}

pub enum RecordOrVariant {
    Record(Record),
    Variant(VariantId),
}
