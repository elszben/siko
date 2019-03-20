use crate::location_info::item::LocationId;
use crate::syntax::item_path::ItemPath;

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
pub struct ExportedGroup {
    pub name: String,
    pub members: Vec<ExportedMember>,
}

#[derive(Debug, Clone)]
pub struct HiddenItem {
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum ExportList {
    ImplicitAll,
    Explicit(Vec<ExportedItem>),
}
