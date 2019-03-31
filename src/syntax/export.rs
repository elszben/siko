use crate::location_info::item::LocationId;

#[derive(Debug, Clone)]
pub enum ExportedItem {
    Named(String),
    Group(ExportedGroup),
}

#[derive(Debug, Clone)]
pub enum ExportedMember {
    Specific(String),
    All,
}

#[derive(Debug, Clone)]
pub struct ExportedMemberInfo {
    pub member: ExportedMember,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub struct ExportedGroup {
    pub name: String,
    pub members: Vec<ExportedMemberInfo>,
}

#[derive(Debug, Clone)]
pub struct ExportedItemInfo {
    pub item: ExportedItem,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub enum ExportList {
    ImplicitAll,
    Explicit(Vec<ExportedItemInfo>),
}
