use crate::name_resolution::error::ResolverError;
use crate::name_resolution::export::ExportedItem;
use crate::name_resolution::import::ImportItemInfo;
use crate::name_resolution::import::ImportMemberInfo;
use crate::name_resolution::import::ImportedItem;
use crate::name_resolution::module::Module;
use crate::syntax::import::ImportKind;
use crate::syntax::import::ImportList;
use crate::syntax::item_path::ItemPath;
use crate::syntax::program::Program;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ImportMode {
    NameAndNamespace,
    NamespaceOnly,
}

fn get_imported_item(exported_item: &ExportedItem) -> ImportedItem {
    match exported_item {
        ExportedItem::Adt(adt_id) => ImportedItem::Adt(*adt_id),
        ExportedItem::Function(function_id) => ImportedItem::Function(*function_id),
        ExportedItem::Record(record_id) => ImportedItem::Record(*record_id),
    }
}

fn get_imported_item_info(imported_item: ImportedItem, source_module: &ItemPath) -> ImportItemInfo {
    ImportItemInfo {
        item: imported_item,
        source_module: source_module.clone(),
    }
}

fn get_names(namespace: &str, item_name: &str, mode: ImportMode) -> Vec<String> {
    match mode {
        ImportMode::NamespaceOnly => vec![format!("{}.{}", namespace, item_name)],
        ImportMode::NameAndNamespace => vec![
            item_name.to_string(),
            format!("{}.{}", namespace, item_name),
        ],
    }
}

fn is_hidden(
    item_name: &str,
    source_module: &String,
    all_hidden_items: &BTreeMap<String, Vec<String>>,
) -> bool {
    match all_hidden_items.get(item_name) {
        Some(items) => items.contains(source_module),
        None => false,
    }
}

pub fn process_imports(
    modules: &mut BTreeMap<String, Module>,
    program: &Program,
    errors: &mut Vec<ResolverError>,
) {
    let mut all_imported_items = Vec::new();
    let mut all_imported_members = Vec::new();

    for (module_name, module) in modules.iter() {
        println!("Processing imports for module {}", module_name);
        let mut all_hidden_items = BTreeMap::new();
        let mut imported_items: BTreeMap<String, Vec<ImportItemInfo>> = BTreeMap::new();
        let mut imported_members: BTreeMap<String, Vec<ImportMemberInfo>> = BTreeMap::new();
        let ast_module = program.modules.get(&module.id).expect("Module not found");
        for (import_id, import) in &ast_module.imports {
            let source_module = match modules.get(&import.module_path.get()) {
                Some(source_module) => source_module,
                None => {
                    let err = ResolverError::ImportedModuleNotFound(
                        import.module_path.get(),
                        import.location_id,
                    );
                    errors.push(err);
                    continue;
                }
            };
            match &import.kind {
                ImportKind::Hiding(hidden_items) => {
                    for item in hidden_items {
                        let mut found = false;
                        if source_module.exported_items.get(&item.name).is_some() {
                            found = true;
                        }
                        if source_module.exported_members.get(&item.name).is_some() {
                            found = true;
                        }
                        if !found {
                            let err = ResolverError::ImportedSymbolNotExportedByModule(
                                item.name.clone(),
                                import.module_path.get(),
                                import.location_id,
                            );
                            errors.push(err);
                        } else {
                            let hs = all_hidden_items
                                .entry(item.name.clone())
                                .or_insert_with(|| Vec::new());
                            hs.push(import.module_path.get());
                        }
                    }
                }
                ImportKind::ImportList { .. } => {}
            }
        }

        for (import_id, import) in &ast_module.imports {
            let source_module = match modules.get(&import.module_path.get()) {
                Some(source_module) => source_module,
                None => {
                    continue;
                }
            };
            let source_module_name = source_module.name.get();
            match &import.kind {
                ImportKind::Hiding(..) => {}
                ImportKind::ImportList {
                    items,
                    alternative_name,
                } => {
                    let (namespace, mode) = match &alternative_name {
                        Some(n) => (n.clone(), ImportMode::NamespaceOnly),
                        None => (import.module_path.get(), ImportMode::NameAndNamespace),
                    };
                    println!("Namespace settings {} {:?}", namespace, mode);
                    match items {
                        ImportList::ImplicitAll => {
                            for (item_name, exported_item) in &source_module.exported_items {
                                if is_hidden(item_name, &source_module_name, &all_hidden_items) {
                                    continue;
                                }
                                let imported_item = get_imported_item(exported_item);
                                let imported_item_info =
                                    get_imported_item_info(imported_item, &source_module.name);
                                let names = get_names(&namespace, item_name, mode);
                                for name in names {
                                    println!("Importing {}", name);
                                }
                            }
                        }
                        ImportList::Explicit(imported_items) => {}
                    }
                }
            }
        }

        println!("Module {} imports:", module_name);
        println!(
            "{} imported items {} imported members",
            imported_items.len(),
            imported_members.len(),
        );
        for (name, import) in &imported_items {
            println!("Item: {} => {:?}", name, import);
        }
        for (name, import) in &imported_members {
            println!("Member: {} => {:?}", name, import);
        }

        all_imported_items.push((module_name, imported_items));
        all_imported_members.push((module_name, imported_members));
    }
}
