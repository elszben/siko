use crate::syntax::item_path::ItemPath;

#[derive(Debug, Clone)]
pub struct Import {
    pub id: ImportId,
    pub module_path: ItemPath,
    pub alternative_name: Option<String>,
    pub symbols: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ImportId {
    pub id: usize,
}
