use crate::location_info::item::LocationId;
use crate::syntax::item_path::ItemPath;

#[derive(Debug, Clone)]
pub enum ImportedItem {
    NamedItem(String),
    Group(ImportedGroup),
}

#[derive(Debug, Clone)]
pub enum ImportedMember {
    Specific(String),
    All,
}

#[derive(Debug, Clone)]
pub struct ImportedGroup {
    pub name: String,
    pub members: Vec<ImportedMember>,
}

#[derive(Debug, Clone)]
pub struct HiddenItem {
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum ImportKind {
    Hiding(Vec<HiddenItem>),
    ImportList {
        items: ImportList,
        alternative_name: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub enum ImportList {
    ImplicitAll,
    Explicit(Vec<ImportedItem>),
}

#[derive(Debug, Clone)]
pub struct Import {
    pub id: ImportId,
    pub module_path: ItemPath,
    pub kind: ImportKind,
    pub location_id: LocationId,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ImportId {
    pub id: usize,
}
