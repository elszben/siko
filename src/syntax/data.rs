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
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub id: VariantId,
    pub items: Vec<TypeSignatureId>,
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
    pub name: String,
    pub id: RecordId,
    pub items: Vec<RecordItem>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct RecordId {
    pub id: usize,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct RecordItemId {
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct RecordItem {
    pub name: String,
    pub id: RecordItemId,
    pub type_signature_id: TypeSignatureId,
}

pub enum RecordOrVariant {
    Record(Record),
    Variant(VariantId),
}
