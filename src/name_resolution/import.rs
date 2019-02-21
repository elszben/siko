use crate::ir::NamedFunctionId;
use crate::syntax::item_path::ItemPath;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ImportInfo {
    source_module: ItemPath,
    function: String,
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ImportStore {
    imported_functions: BTreeMap<String, BTreeSet<ImportInfo>>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum ImportKind {
    NameOnly,
    NameAndNamespace,
    NamespaceOnly,
}

impl ImportStore {
    pub fn new() -> ImportStore {
        ImportStore {
            imported_functions: BTreeMap::new(),
        }
    }

    pub fn extend(&mut self, other: ImportStore) {
        for (func, infos) in other.imported_functions {
            let is = self
                .imported_functions
                .entry(func)
                .or_insert_with(BTreeSet::new);
            is.extend(infos);
        }
    }

    pub fn add_imported_function(
        &mut self,
        func_name: String,
        source_module: ItemPath,
        namespace: String,
        kind: ImportKind,
    ) {
        let qualified_form = format!("{}.{}", namespace, func_name);
        let short = func_name.clone();
        let names = match kind {
            ImportKind::NameOnly => vec![short],
            ImportKind::NameAndNamespace => vec![qualified_form, short],
            ImportKind::NamespaceOnly => vec![qualified_form],
        };
        for name in names {
            let infos = self
                .imported_functions
                .entry(name)
                .or_insert_with(BTreeSet::new);
            let info = ImportInfo {
                source_module: source_module.clone(),
                function: func_name.clone(),
            };
            infos.insert(info);
        }
    }

    pub fn get_function_id(&self, path: &String) -> Vec<(String, String)> {
        let mut result = Vec::new();
        if let Some(infos) = self.imported_functions.get(path) {
            for i in infos.iter() {
                let function_id = (i.source_module.get(), i.function.clone());
                result.push(function_id);
            }
        }
        result
    }
}
