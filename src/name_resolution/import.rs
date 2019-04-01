use crate::name_resolution::item::DataMember;
use crate::name_resolution::item::Item;

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
