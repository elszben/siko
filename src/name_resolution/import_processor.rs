use crate::name_resolution::error::ResolverError;
use crate::name_resolution::import::ImportItemInfo;
use crate::name_resolution::import::ImportMemberInfo;
use crate::name_resolution::module::Module;
use crate::syntax::import::ImportKind;
use crate::syntax::program::Program;
use std::collections::BTreeMap;

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
                        println!("{} is hidden from {}", item.name, import.module_path.get());
                        let hs = all_hidden_items
                            .entry(item.name.clone())
                            .or_insert_with(|| Vec::new());
                        hs.push(import.module_path.get());
                    }
                }
                _ => {}
            }
        }

        for (import_id, import) in &ast_module.imports {
            match &import.kind {
                ImportKind::Hiding(hidden_items) => {}
                ImportKind::ImportList {
                    items,
                    alternative_name,
                } => {}
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
