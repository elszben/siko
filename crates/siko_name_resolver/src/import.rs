use crate::item::DataMember;
use crate::item::Item;

#[derive(Debug, Clone)]
pub struct ImportedItemInfo {
    pub item: Item,
    pub source_module: String,
}

#[derive(Debug, Clone)]
pub struct ImportedMemberInfo {
    pub member: DataMember,
    pub source_module: String,
}
