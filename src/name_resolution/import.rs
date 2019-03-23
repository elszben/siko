use crate::syntax::data::AdtId;
use crate::syntax::data::RecordFieldId;
use crate::syntax::data::RecordId;
use crate::syntax::data::VariantId;
use crate::syntax::function::FunctionId;
use crate::syntax::item_path::ItemPath;

#[derive(Debug)]
pub enum ImportedItem {
    Function(FunctionId),
    Record(RecordId),
    Variant(VariantId),
    Adt(AdtId),
}

#[derive(Debug)]
pub enum ImportedDataMember {
    RecordField(ImportedField),
    Variant(ImportedVariant),
}

#[derive(Debug)]
pub struct ImportedField {
    pub field_id: RecordFieldId,
    pub record_id: RecordId,
}

#[derive(Debug)]
pub struct ImportedVariant {
    pub variant_id: VariantId,
    pub adt_id: AdtId,
}

#[derive(Debug)]
pub struct ImportItemInfo {
    pub item: ImportedItem,
    pub source_module: ItemPath,
}

#[derive(Debug)]
pub struct ImportMemberInfo {
    pub member: ImportedDataMember,
    pub source_module: ItemPath,
    pub hidden: bool,
}
