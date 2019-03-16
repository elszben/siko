use crate::location_info::item::LocationId;
use crate::syntax::item_path::ItemPath;

#[derive(Debug, Clone)]
pub enum ExportedItem {
    NamedItem(String),
    TypeConstructor(ExportedTypeConstructor),
}

#[derive(Debug, Clone)]
pub enum ExportedDataConstructor {
    Specific(String),
    All,
}

#[derive(Debug, Clone)]
pub struct ExportedTypeConstructor {
    pub name: String,
    pub data_constructors: Vec<ExportedDataConstructor>,
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
