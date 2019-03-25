use crate::ir::types::TypeDefId;
use crate::syntax::data::AdtId;
use crate::syntax::data::RecordFieldId;
use crate::syntax::data::RecordId;
use crate::syntax::data::VariantId;
use crate::syntax::function::FunctionId;

#[derive(Debug, Clone)]
pub enum ImportedItem {
    Function(FunctionId),
    Record(RecordId, TypeDefId),
    Adt(AdtId, TypeDefId),
}

#[derive(Debug, Clone)]
pub enum ImportedDataMember {
    RecordField(ImportedField),
    Variant(ImportedVariant),
}

#[derive(Debug, Clone)]
pub struct ImportedField {
    pub field_id: RecordFieldId,
    pub record_id: RecordId,
}

#[derive(Debug, Clone)]
pub struct ImportedVariant {
    pub variant_id: VariantId,
    pub adt_id: AdtId,
}

#[derive(Debug, Clone)]
pub struct ImportedItemInfo {
    pub item: ImportedItem,
    pub source_module: String,
}

#[derive(Debug, Clone)]
pub struct ImportedMemberInfo {
    pub member: ImportedDataMember,
    pub source_module: String,
}
