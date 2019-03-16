use crate::syntax::data::AdtId;
use crate::syntax::data::RecordId;
use crate::syntax::data::VariantId;
use crate::syntax::function::FunctionId;
use crate::syntax::item_path::ItemPath;

#[derive(Debug)]
pub enum ImportedItem {
    Function(FunctionId),
    Record(RecordId),
    DataConstructor(VariantId),
}

#[derive(Debug)]
pub enum ImportedType {
    Record(RecordId),
    TypeConstructor(AdtId),
}

#[derive(Debug)]
pub struct ImportItemInfo {
    pub item: ImportedItem,
    pub source_module: ItemPath,
}

#[derive(Debug)]
pub struct ImportTypeInfo {
    pub item: ImportedType,
    pub source_module: ItemPath,
}
