use crate::name_resolution::error::ResolverError;
use crate::name_resolution::export_import_pattern::check_item;
use crate::name_resolution::export_import_pattern::check_member;
use crate::name_resolution::export_import_pattern::process_patterns;
use crate::name_resolution::export_import_pattern::MemberPatternKind;
use crate::name_resolution::item::Item;
use crate::name_resolution::module::Module;
use crate::syntax::program::Program;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub fn process_exports(
    modules: &mut BTreeMap<String, Module>,
    program: &Program,
    errors: &mut Vec<ResolverError>,
) {
    for (module_name, module) in modules.iter_mut() {
        let mut exported_items = BTreeMap::new();
        let mut exported_members = BTreeMap::new();
        let mut matched_classes = BTreeSet::new();
        let ast_module = program.modules.get(&module.id).expect("Module not found");

        let (mut item_patterns, mut member_patterns) = process_patterns(&ast_module.export_list);

        for (item_name, items) in &module.items {
            assert_eq!(items.len(), 1);
            let item = &items[0];
            check_item(
                &mut item_patterns,
                &mut member_patterns,
                item_name,
                item,
                program,
                &mut exported_items,
                &mut matched_classes,
            );
        }

        for (name, items) in &module.items {
            assert_eq!(items.len(), 1);
            let item = &items[0];
            if let Item::ClassMember(class_id, _, _) = item {
                if matched_classes.contains(class_id) {
                    exported_items.insert(name.clone(), item.clone());
                }
            }
        }

        for pattern in item_patterns {
            match &pattern.name {
                Some(name) => {
                    if !pattern.matched {
                        let err = ResolverError::ExportNoMatch(
                            module_name.clone(),
                            name.clone(),
                            pattern.location_id.expect("No location"),
                        );
                        errors.push(err);
                    }
                }
                None => {}
            }
        }

        for (member_name, members) in &module.members {
            for member in members {
                check_member(
                    &mut member_patterns,
                    member_name,
                    member,
                    program,
                    &mut exported_members,
                );
            }
        }

        for pattern_kind in member_patterns {
            match pattern_kind {
                MemberPatternKind::ImplicitAll => {}
                MemberPatternKind::Specific(pattern) => match &pattern.name {
                    Some(name) => {
                        if !pattern.matched {
                            let err = ResolverError::ExportNoMatch(
                                module_name.clone(),
                                name.clone(),
                                pattern.location_id,
                            );
                            errors.push(err);
                        }
                    }
                    None => {}
                },
            }
        }

        module.exported_items = exported_items;
        module.exported_members = exported_members;

        /*
        println!("Module {} exports:", module_name);
        println!(
            "{} exported items {} exported members",
            module.exported_items.len(),
            module.exported_members.len(),
        );
        for (name, export) in &module.exported_items {
            println!("Item: {} => {:?}", name, export);
        }
        for (name, export) in &module.exported_members {
            println!("Member: {} => {:?}", name, export);
        }
        */
    }
}
