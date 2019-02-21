use crate::name_resolution::import::ImportStore;
use crate::syntax::function::Function;
use crate::syntax::item_path::ItemPath;
use crate::syntax::module::ModuleId;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Module<'a> {
    pub id: ModuleId,
    pub name: ItemPath,
    pub exported_functions: BTreeMap<String, Vec<(&'a Function)>>,
    pub imported_functions: ImportStore,
}

impl<'a> Module<'a> {
    pub fn new(id: ModuleId, name: ItemPath) -> Module<'a> {
        Module {
            id: id,
            name: name,
            exported_functions: BTreeMap::new(),
            imported_functions: ImportStore::new(),
        }
    }
}
