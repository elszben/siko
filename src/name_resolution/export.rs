use crate::syntax::data::AdtId;
use crate::syntax::data::RecordFieldId;
use crate::syntax::data::RecordId;
use crate::syntax::data::VariantId;
use crate::syntax::function::FunctionId;

#[derive(Debug)]
pub enum ExportedItem {
    Function(FunctionId),
    Record(RecordId),
    Adt(AdtId),
}

#[derive(Debug)]
pub enum ExportedDataMember {
    RecordField(ExportedField),
    Variant(ExportedVariant),
}

#[derive(Debug)]
pub struct ExportedField {
    pub field_id: RecordFieldId,
    pub record_id: RecordId,
}

#[derive(Debug)]
pub struct ExportedVariant {
    pub variant_id: VariantId,
    pub adt_id: AdtId,
}
