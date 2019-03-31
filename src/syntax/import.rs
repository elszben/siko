use crate::location_info::item::LocationId;
use crate::syntax::export_import::EIItemInfo;

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
    Explicit(Vec<EIItemInfo>),
}

#[derive(Debug, Clone)]
pub struct Import {
    pub id: ImportId,
    pub module_path: String,
    pub kind: ImportKind,
    pub location_id: Option<LocationId>,
}

impl Import {
    pub fn get_location(&self) -> LocationId {
        self.location_id
            .expect("Trying to get the location of an internal import")
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ImportId {
    pub id: usize,
}
