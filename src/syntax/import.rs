use crate::syntax::item_path::ItemPath;

#[derive(Debug, Clone)]
pub enum ImportedItem {
    FunctionOrRecord(String),
    TypeConstructor(TypeConstructor),
}

#[derive(Debug, Clone)]
pub struct DataConstructor {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TypeConstructor {
    pub name: String,
    pub data_constructors: Vec<DataConstructor>,
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
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ImportId {
    pub id: usize,
}
