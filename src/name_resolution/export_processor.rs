use crate::location_info::item::LocationId;
use crate::name_resolution::error::ResolverError;
use crate::name_resolution::item::DataMember;
use crate::name_resolution::item::Item;
use crate::name_resolution::module::Module;
use crate::syntax::data::AdtId;
use crate::syntax::data::RecordId;
use crate::syntax::export::ExportList;
use crate::syntax::export::ExportedItem as AstExportedItem;
use crate::syntax::export::ExportedMember;
use crate::syntax::module::Module as AstModule;
use crate::syntax::program::Program;
use std::collections::BTreeMap;

struct ExportItemPattern {
    name: Option<String>,
    group: bool,
    matched: bool,
    location_id: Option<LocationId>,
}

impl ExportItemPattern {
    fn new(
        name: Option<String>,
        group: bool,
        location_id: Option<LocationId>,
    ) -> ExportItemPattern {
        ExportItemPattern {
            name: name,
            group: group,
            matched: false,
            location_id: location_id,
        }
    }
}

struct ExportMemberPattern {
    group_name: String,
    name: Option<String>,
    matched: bool,
    location_id: LocationId,
}

enum ExportMemberPatternKind {
    ImplicitAll,
    Specific(ExportMemberPattern),
}

impl ExportMemberPattern {
    fn new(
        group_name: String,
        name: Option<String>,
        location_id: LocationId,
    ) -> ExportMemberPattern {
        ExportMemberPattern {
            group_name: group_name,
            name: name,
            matched: false,
            location_id: location_id,
        }
    }
}

fn process_patterns(
    ast_module: &AstModule,
) -> (Vec<ExportItemPattern>, Vec<ExportMemberPatternKind>) {
    let mut item_patterns = Vec::new();
    let mut member_patterns = Vec::new();
    match &ast_module.export_list {
        ExportList::ImplicitAll => {
            item_patterns.push(ExportItemPattern::new(None, false, None));
            member_patterns.push(ExportMemberPatternKind::ImplicitAll);
        }
        ExportList::Explicit(pattern_items) => {
            for pattern_item in pattern_items {
                let item = &pattern_item.item;
                match item {
                    AstExportedItem::Named(entity_name) => {
                        item_patterns.push(ExportItemPattern::new(
                            Some(entity_name.clone()),
                            false,
                            Some(pattern_item.location_id),
                        ));
                    }
                    AstExportedItem::Group(group) => {
                        item_patterns.push(ExportItemPattern::new(
                            Some(group.name.clone()),
                            true,
                            Some(pattern_item.location_id),
                        ));
                        for member_info in &group.members {
                            match &member_info.member {
                                ExportedMember::All => {
                                    let pattern = ExportMemberPattern::new(
                                        group.name.clone(),
                                        None,
                                        member_info.location_id,
                                    );
                                    member_patterns
                                        .push(ExportMemberPatternKind::Specific(pattern));
                                }
                                ExportedMember::Specific(name) => {
                                    let pattern = ExportMemberPattern::new(
                                        group.name.clone(),
                                        Some(name.clone()),
                                        member_info.location_id,
                                    );
                                    member_patterns
                                        .push(ExportMemberPatternKind::Specific(pattern));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    (item_patterns, member_patterns)
}

fn match_item(name: &str, group: bool, item: &Item, program: &Program) -> bool {
    match item {
        Item::Function(id, _) => {
            let function = program.functions.get(&id).expect("Function not found");
            function.name == name && !group
        }
        Item::Record(id, _) => {
            let record = program.records.get(&id).expect("Record not found");
            record.name == name
        }
        Item::Adt(id, _) => {
            let adt = program.adts.get(&id).expect("Adt not found");
            adt.name == name
        }
        Item::Variant(..) => {
            // cannot match on a single variant
            false
        }
    }
}

fn match_member(
    group_name: &str,
    name: Option<&String>,
    member: &DataMember,
    program: &Program,
) -> bool {
    match member {
        DataMember::RecordField(field) => {
            let record = program
                .records
                .get(&field.record_id)
                .expect("Record not found");
            for record_field in &record.fields {
                if record.name == group_name {
                    if let Some(n) = name {
                        if *n == record_field.name {
                            return true;
                        }
                    } else {
                        return true;
                    }
                }
            }
        }
        DataMember::Variant(variant) => {
            let adt = program.adts.get(&variant.adt_id).expect("Adt not found");
            let ast_variant = program
                .variants
                .get(&variant.variant_id)
                .expect("Variant not found");
            if adt.name == group_name {
                if let Some(n) = name {
                    if *n == ast_variant.name {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }
    }
    false
}

fn check_item(
    item_patterns: &mut Vec<ExportItemPattern>,
    member_patterns: &mut Vec<ExportMemberPatternKind>,
    item_name: &str,
    item: &Item,
    program: &Program,
    exported_items: &mut BTreeMap<String, Item>,
) {
    let mut exported_item = false;
    for pattern in item_patterns.iter_mut() {
        match &pattern.name {
            Some(name) => {
                if match_item(name, pattern.group, item, program) {
                    exported_item = true;
                    pattern.matched = true;
                }
            }
            None => {
                // implicit
                exported_item = true;
            }
        }
    }
    for pattern_kind in member_patterns.iter_mut() {
        match item {
            Item::Variant(adt_id, variant_id) => match pattern_kind {
                ExportMemberPatternKind::ImplicitAll => {
                    exported_item = true;
                }
                ExportMemberPatternKind::Specific(pattern) => {
                    let adt = program.adts.get(&adt_id).expect("Adt not found");
                    let ast_variant = program
                        .variants
                        .get(&variant_id)
                        .expect("Variant not found");
                    if adt.name == pattern.group_name {
                        if let Some(n) = &pattern.name {
                            if *n == ast_variant.name {
                                exported_item = true;
                            }
                        } else {
                            exported_item = true;
                        }
                    }
                }
            },
            _ => {}
        }
    }
    if exported_item {
        exported_items.insert(item_name.to_string(), item.clone());
    }
}

fn check_member(
    member_patterns: &mut Vec<ExportMemberPatternKind>,
    member_name: &str,
    member: &DataMember,
    program: &Program,
    exported_members: &mut BTreeMap<String, Vec<DataMember>>,
) {
    let mut exported_member = false;
    for pattern_kind in member_patterns.iter_mut() {
        match pattern_kind {
            ExportMemberPatternKind::ImplicitAll => exported_member = true,
            ExportMemberPatternKind::Specific(pattern) => {
                if match_member(&pattern.group_name, pattern.name.as_ref(), member, program) {
                    exported_member = true;
                    pattern.matched = true;
                }
            }
        }
    }
    if exported_member {
        let members = exported_members
            .entry(member_name.to_string())
            .or_insert_with(|| Vec::new());
        members.push(member.clone());
    }
}

pub fn process_exports(
    modules: &mut BTreeMap<String, Module>,
    program: &Program,
    errors: &mut Vec<ResolverError>,
) {
    for (module_name, module) in modules.iter_mut() {
        let mut exported_items = BTreeMap::new();
        let mut exported_members = BTreeMap::new();
        let ast_module = program.modules.get(&module.id).expect("Module not found");

        let (mut item_patterns, mut member_patterns) = process_patterns(ast_module);

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
            );
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
                ExportMemberPatternKind::ImplicitAll => {}
                ExportMemberPatternKind::Specific(pattern) => match &pattern.name {
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
    }
}
