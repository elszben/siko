use crate::syntax::function::Function;
use crate::syntax::function::FunctionId;
use crate::syntax::import::Import;
use crate::syntax::import::ImportId;
use crate::syntax::item_path::ItemPath;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ModuleId {
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: ItemPath,
    pub id: ModuleId,
    pub functions: BTreeMap<FunctionId, Function>,
    pub imports: BTreeMap<ImportId, Import>,
}

impl Module {
    pub fn new(name: ItemPath, id: ModuleId) -> Module {
        Module {
            name: name,
            id: id,
            functions: BTreeMap::new(),
            imports: BTreeMap::new(),
        }
    }

    pub fn add_function(&mut self, function_id: FunctionId, function: Function) {
        self.functions.insert(function_id, function);
    }

    pub fn add_import(&mut self, import_id: ImportId, import: Import) {
        self.imports.insert(import_id, import);
    }
}
