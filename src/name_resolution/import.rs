use crate::syntax::data::AdtId;
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
pub struct ImportItemInfo {
    pub item: ImportedItem,
    pub source_module: ItemPath,
}
