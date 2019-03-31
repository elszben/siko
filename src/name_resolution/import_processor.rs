use crate::name_resolution::error::ResolverError;
use crate::name_resolution::import::ImportedDataMember;
use crate::name_resolution::import::ImportedField;
use crate::name_resolution::import::ImportedItemInfo;
use crate::name_resolution::import::ImportedMemberInfo;
use crate::name_resolution::import::ImportedVariant;
use crate::name_resolution::item::Item;
use crate::name_resolution::module::Module;
use crate::syntax::data::AdtId;
use crate::syntax::data::RecordId;
use crate::syntax::import::Import;
use crate::syntax::import::ImportKind;
use crate::syntax::import::ImportList;
use crate::syntax::import::ImportedGroup;
use crate::syntax::import::ImportedItem as AstImportedItem;
use crate::syntax::import::ImportedMember;
use crate::syntax::program::Program;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ImportMode {
    NameAndNamespace,
    NamespaceOnly,
}

fn get_imported_item_info(imported_item: Item, source_module: String) -> ImportedItemInfo {
    ImportedItemInfo {
        item: imported_item,
        source_module: source_module,
    }
}

fn get_imported_member_info(
    imported_member: ImportedDataMember,
    source_module: String,
) -> ImportedMemberInfo {
    ImportedMemberInfo {
        member: imported_member,
        source_module: source_module,
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

fn process_record_field_import_list(
    record_id: RecordId,
    group: &ImportedGroup,
    source_module: &Module,
    import: &Import,
    module: &Module,
    errors: &mut Vec<ResolverError>,
    all_hidden_items: &BTreeMap<String, Vec<String>>,
    imported_members: &mut BTreeMap<String, Vec<ImportedMemberInfo>>,
    namespace: &str,
    mode: ImportMode,
) {
    /*
    for member in &group.members {
        match member {
            ImportedMember::All => {
                for (exported_member_name, source_items) in &source_module.exported_members {
                    if is_hidden(exported_member_name, &source_module.name, &all_hidden_items) {
                        continue;
                    }
                    for source_item in source_items {
                        if let ExportedDataMember::RecordField(exported_field) = source_item {
                            if exported_field.record_id == record_id {
                                import_exported_member(
                                    exported_member_name,
                                    source_item,
                                    source_module,
                                    imported_members,
                                    namespace,
                                    mode,
                                );
                                break;
                            }
                        }
                    }
                }
            }
            ImportedMember::Specific(member_name) => {
                if is_hidden(member_name, &source_module.name, &all_hidden_items) {
                    let err = ResolverError::ExplicitlyImportedRecordFieldHidden(
                        member_name.clone(),
                        module.name.clone(),
                        import.get_location(),
                    );
                    errors.push(err);
                    continue;
                }
                match source_module.exported_members.get(member_name) {
                    Some(source_items) => {
                        let mut found = false;
                        for source_item in source_items {
                            if let ExportedDataMember::RecordField(exported_field) = source_item {
                                if exported_field.record_id == record_id {
                                    found = true;
                                    import_exported_member(
                                        member_name,
                                        source_item,
                                        source_module,
                                        imported_members,
                                        namespace,
                                        mode,
                                    );
                                    break;
                                }
                            }
                        }
                        if !found {
                            let err = ResolverError::ImportedRecordFieldNotExported(
                                group.name.clone(),
                                member_name.clone(),
                                import.get_location(),
                            );
                            errors.push(err);
                        }
                    }
                    None => {
                        let err = ResolverError::ImportedRecordFieldNotExported(
                            group.name.clone(),
                            member_name.clone(),
                            import.get_location(),
                        );
                        errors.push(err);
                    }
                }
            }
        }
    }
    */
}

fn process_adt_variant_import_list(
    adt_id: AdtId,
    group: &ImportedGroup,
    source_module: &Module,
    import: &Import,
    module: &Module,
    errors: &mut Vec<ResolverError>,
    all_hidden_items: &BTreeMap<String, Vec<String>>,
    imported_members: &mut BTreeMap<String, Vec<ImportedMemberInfo>>,
    namespace: &str,
    mode: ImportMode,
) {
    /*
    for member in &group.members {
        match member {
            ImportedMember::All => {
                for (exported_member_name, source_items) in &source_module.exported_members {
                    if is_hidden(exported_member_name, &source_module.name, &all_hidden_items) {
                        continue;
                    }
                    for source_item in source_items {
                        if let ExportedDataMember::Variant(exported_variant) = source_item {
                            if exported_variant.adt_id == adt_id {
                                import_exported_member(
                                    exported_member_name,
                                    source_item,
                                    source_module,
                                    imported_members,
                                    namespace,
                                    mode,
                                );
                                break;
                            }
                        }
                    }
                }
            }
            ImportedMember::Specific(member_name) => {
                if is_hidden(member_name, &source_module.name, &all_hidden_items) {
                    let err = ResolverError::ExplicitlyImportedAdtVariantdHidden(
                        member_name.clone(),
                        module.name.clone(),
                        import.get_location(),
                    );
                    errors.push(err);
                    continue;
                }
                match source_module.exported_members.get(member_name) {
                    Some(source_items) => {
                        let mut found = false;
                        for source_item in source_items {
                            if let ExportedDataMember::Variant(exported_variant) = source_item {
                                if exported_variant.adt_id == adt_id {
                                    found = true;
                                    import_exported_member(
                                        member_name,
                                        source_item,
                                        source_module,
                                        imported_members,
                                        namespace,
                                        mode,
                                    );
                                    break;
                                }
                            }
                        }
                        if !found {
                            let err = ResolverError::ImportedAdtVariantNotExported(
                                group.name.clone(),
                                member_name.clone(),
                                import.get_location(),
                            );
                            errors.push(err);
                        }
                    }
                    None => {
                        let err = ResolverError::ImportedAdtVariantNotExported(
                            group.name.clone(),
                            member_name.clone(),
                            import.get_location(),
                        );
                        errors.push(err);
                    }
                }
            }
        }
    }
    */
}

fn process_explicit_import_list(
    import: &Import,
    imported_list_items: &Vec<AstImportedItem>,
    source_module: &Module,
    module: &Module,
    errors: &mut Vec<ResolverError>,
    all_hidden_items: &BTreeMap<String, Vec<String>>,
    imported_items: &mut BTreeMap<String, Vec<ImportedItemInfo>>,
    imported_members: &mut BTreeMap<String, Vec<ImportedMemberInfo>>,
    namespace: &str,
    mode: ImportMode,
) {
    for imported_list_item in imported_list_items {
        match imported_list_item {
            AstImportedItem::Group(group) => {
                if is_hidden(&group.name, &source_module.name, &all_hidden_items) {
                    let err = ResolverError::ExplicitlyImportedTypeHidden(
                        group.name.clone(),
                        module.name.clone(),
                        import.get_location(),
                    );
                    errors.push(err);
                    continue;
                }
                match source_module.exported_items.get(&group.name) {
                    Some(item) => match item {
                        Item::Record(record_id, _) => {
                            import_exported_item(
                                &group.name,
                                &item,
                                source_module,
                                imported_items,
                                namespace,
                                mode,
                            );
                            process_record_field_import_list(
                                *record_id,
                                group,
                                source_module,
                                import,
                                module,
                                errors,
                                all_hidden_items,
                                imported_members,
                                namespace,
                                mode,
                            );
                        }
                        Item::Adt(adt_id, _) => {
                            import_exported_item(
                                &group.name,
                                &item,
                                source_module,
                                imported_items,
                                namespace,
                                mode,
                            );
                            process_adt_variant_import_list(
                                *adt_id,
                                group,
                                source_module,
                                import,
                                module,
                                errors,
                                all_hidden_items,
                                imported_members,
                                namespace,
                                mode,
                            );
                        }
                        Item::Function(..) | Item::Variant(..) => {
                            let err = ResolverError::IncorrectNameInImportedTypeConstructor(
                                import.module_path.clone(),
                                group.name.clone(),
                                import.get_location(),
                            );
                            errors.push(err);
                        }
                    },
                    None => {
                        let err = ResolverError::IncorrectNameInImportedTypeConstructor(
                            import.module_path.clone(),
                            group.name.clone(),
                            import.get_location(),
                        );
                        errors.push(err);
                    }
                }
            }
            AstImportedItem::NamedItem(item_name) => {
                if is_hidden(item_name, &source_module.name, &all_hidden_items) {
                    let err = ResolverError::ExplicitlyImportedItemHidden(
                        item_name.clone(),
                        module.name.clone(),
                        import.get_location(),
                    );
                    errors.push(err);
                    continue;
                }

                match source_module.exported_items.get(item_name) {
                    Some(exported_item) => {
                        let imported_item_info = get_imported_item_info(
                            exported_item.clone(),
                            source_module.name.clone(),
                        );
                        let names = get_names(&namespace, item_name, mode);
                        for name in names {
                            let imported_item_infos = imported_items
                                .entry(name.clone())
                                .or_insert_with(|| Vec::new());
                            imported_item_infos.push(imported_item_info.clone());
                        }
                    }
                    None => {
                        let err = ResolverError::ImportedSymbolNotExportedByModule(
                            item_name.clone(),
                            import.module_path.clone(),
                            import.get_location(),
                        );
                        errors.push(err);
                    }
                }
            }
        }
    }
}

fn import_exported_member(
    member_name: &str,
    source_module: &Module,
    imported_members: &mut BTreeMap<String, Vec<ImportedMemberInfo>>,
    namespace: &str,
    mode: ImportMode,
) {
}

fn import_exported_item(
    item_name: &str,
    exported_item: &Item,
    source_module: &Module,
    imported_items: &mut BTreeMap<String, Vec<ImportedItemInfo>>,
    namespace: &str,
    mode: ImportMode,
) {
    let imported_item_info =
        get_imported_item_info(exported_item.clone(), source_module.name.clone());
    let names = get_names(&namespace, item_name, mode);
    for name in names {
        let imported_item_infos = imported_items
            .entry(name.clone())
            .or_insert_with(|| Vec::new());
        imported_item_infos.push(imported_item_info.clone());
    }
}

fn process_implicit_import_list(
    source_module: &Module,
    all_hidden_items: &BTreeMap<String, Vec<String>>,
    imported_items: &mut BTreeMap<String, Vec<ImportedItemInfo>>,
    imported_members: &mut BTreeMap<String, Vec<ImportedMemberInfo>>,
    namespace: &str,
    mode: ImportMode,
) {
    for (item_name, exported_item) in &source_module.exported_items {
        if is_hidden(item_name, &source_module.name, &all_hidden_items) {
            continue;
        }
        import_exported_item(
            item_name,
            exported_item,
            source_module,
            imported_items,
            namespace,
            mode,
        );
    }
    for (member_name, exported_members) in &source_module.exported_members {
        if is_hidden(member_name, &source_module.name, &all_hidden_items) {
            continue;
        }
        for exported_member in exported_members {
            /*
            import_exported_member(
                member_name,
                exported_member,
                source_module,
                imported_members,
                namespace,
                mode,
            );
            */
        }
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
        // println!("Processing imports for module {}", module_name);
        let mut all_hidden_items = BTreeMap::new();
        let mut imported_items = BTreeMap::new();
        let mut imported_members = BTreeMap::new();

        for (name, items) in &module.items {
            let names = get_names(module_name, name, ImportMode::NameAndNamespace);
            let item = &items[0];
            match item {
                Item::Adt(adt_id, _) => {
                    let adt = program.adts.get(adt_id).expect("Adt not found");
                    for variant_id in &adt.variants {
                        let imported_member = ImportedDataMember::Variant(ImportedVariant {
                            adt_id: *adt_id,
                            variant_id: *variant_id,
                        });
                        let variant = program.variants.get(variant_id).expect("Variant not found");
                        let names =
                            get_names(module_name, &variant.name, ImportMode::NameAndNamespace);
                        for name in &names {
                            let ims = imported_members
                                .entry(name.clone())
                                .or_insert_with(|| Vec::new());
                            ims.push(ImportedMemberInfo {
                                member: imported_member.clone(),
                                source_module: module_name.clone(),
                            })
                        }
                    }
                }
                Item::Function(..) | Item::Variant(..) => {}
                Item::Record(record_id, _) => {
                    let record = program.records.get(record_id).expect("Record not found");
                    for field in &record.fields {
                        let imported_member = ImportedDataMember::RecordField(ImportedField {
                            record_id: *record_id,
                            field_id: field.id,
                        });
                        let names =
                            get_names(module_name, &field.name, ImportMode::NameAndNamespace);
                        for name in &names {
                            let ims = imported_members
                                .entry(name.clone())
                                .or_insert_with(|| Vec::new());
                            ims.push(ImportedMemberInfo {
                                member: imported_member.clone(),
                                source_module: module_name.clone(),
                            })
                        }
                    }
                }
            }

            for name in &names {
                let iis = imported_items
                    .entry(name.clone())
                    .or_insert_with(|| Vec::new());
                iis.push(ImportedItemInfo {
                    item: item.clone(),
                    source_module: module_name.clone(),
                })
            }
        }

        let ast_module = program.modules.get(&module.id).expect("Module not found");
        for (_, import) in &ast_module.imports {
            let source_module = match modules.get(&import.module_path) {
                Some(source_module) => source_module,
                None => {
                    let err = ResolverError::ImportedModuleNotFound(
                        import.module_path.clone(),
                        import.get_location(),
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
                                import.module_path.clone(),
                                import.get_location(),
                            );
                            errors.push(err);
                        } else {
                            let hs = all_hidden_items
                                .entry(item.name.clone())
                                .or_insert_with(|| Vec::new());
                            hs.push(import.module_path.clone());
                        }
                    }
                }
                ImportKind::ImportList { .. } => {}
            }
        }

        for (_, import) in &ast_module.imports {
            let source_module = match modules.get(&import.module_path.clone()) {
                Some(source_module) => source_module,
                None => {
                    continue;
                }
            };

            match &import.kind {
                ImportKind::Hiding(..) => {}
                ImportKind::ImportList {
                    items,
                    alternative_name,
                } => {
                    let (namespace, mode) = match &alternative_name {
                        Some(n) => (n.clone(), ImportMode::NamespaceOnly),
                        None => (import.module_path.clone(), ImportMode::NameAndNamespace),
                    };
                    match items {
                        ImportList::ImplicitAll => {
                            process_implicit_import_list(
                                source_module,
                                &all_hidden_items,
                                &mut imported_items,
                                &mut imported_members,
                                &namespace,
                                mode,
                            );
                        }
                        ImportList::Explicit(imported_list_items) => {
                            process_explicit_import_list(
                                import,
                                imported_list_items,
                                source_module,
                                module,
                                errors,
                                &all_hidden_items,
                                &mut imported_items,
                                &mut imported_members,
                                &namespace,
                                mode,
                            );
                        }
                    }
                }
            }
        }

        /*
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
        */

        all_imported_items.push((module_name.clone(), imported_items));
        all_imported_members.push((module_name.clone(), imported_members));
    }

    for (module_name, items) in all_imported_items {
        let module = modules.get_mut(&module_name).expect("Module not found");
        module.imported_items = items;
    }

    for (module_name, members) in all_imported_members {
        let module = modules.get_mut(&module_name).expect("Module not found");
        module.imported_members = members;
    }
}
