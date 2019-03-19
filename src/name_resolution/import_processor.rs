use crate::name_resolution::error::ResolverError;
use crate::name_resolution::module::Module;
use crate::syntax::import::ImportKind;
use crate::syntax::program::Program;
use std::collections::BTreeMap;

pub fn process_imports(
    modules: &mut BTreeMap<String, Module>,
    program: &Program,
    errors: &mut Vec<ResolverError>,
) {
    for (name, module) in modules.iter_mut() {
        println!("Processing imports for module {}", name);
        /*let mut imported_items = BTreeMap::new();
        let mut imported_types = BTreeMap::new();
        let mut imported_fields = BTreeMap::new();
        let mut imported_variants = BTreeMap::new();*/
        let ast_module = program.modules.get(&module.id).expect("Module not found");
        for (import_id, import) in &ast_module.imports {
            match &import.kind {
                ImportKind::Hiding(hidden_items) => {}
                ImportKind::ImportList {
                    items,
                    alternative_name,
                } => {}
            }
        }
    }
}
