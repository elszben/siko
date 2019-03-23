use crate::name_resolution::error::ResolverError;
use crate::name_resolution::export::ExportedDataMember;
use crate::name_resolution::export::ExportedField;
use crate::name_resolution::export::ExportedItem;
use crate::name_resolution::export::ExportedVariant;
use crate::name_resolution::import::ImportedDataMember;
use crate::name_resolution::import::ImportedField;
use crate::name_resolution::import::ImportedItem;
use crate::name_resolution::import::ImportedItemInfo;
use crate::name_resolution::import::ImportedMemberInfo;
use crate::name_resolution::import::ImportedVariant;
use crate::name_resolution::module::Module;
use crate::syntax::import::Import;
use crate::syntax::import::ImportKind;
use crate::syntax::import::ImportList;
use crate::syntax::import::ImportedItem as AstImportedItem;
use crate::syntax::import::ImportedMember;
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

fn get_imported_item_info(
    imported_item: ImportedItem,
    source_module: &ItemPath,
) -> ImportedItemInfo {
    ImportedItemInfo {
        item: imported_item,
        source_module: source_module.clone(),
    }
}

fn get_imported_member(exported_member: &ExportedDataMember) -> (ImportedDataMember, bool) {
    match exported_member {
        ExportedDataMember::RecordField(ExportedField {
            field_id,
            record_id,
        }) => (
            ImportedDataMember::RecordField(ImportedField {
                field_id: *field_id,
                record_id: *record_id,
            }),
            true,
        ),
        ExportedDataMember::Variant(ExportedVariant { variant_id, adt_id }) => (
            ImportedDataMember::Variant(ImportedVariant {
                variant_id: *variant_id,
                adt_id: *adt_id,
            }),
            false,
        ),
    }
}

fn get_imported_member_info(
    imported_member: ImportedDataMember,
    source_module: &ItemPath,
) -> ImportedMemberInfo {
    ImportedMemberInfo {
        member: imported_member,
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
    let source_module_name = source_module.name.get();
    for imported_list_item in imported_list_items {
        match imported_list_item {
            AstImportedItem::Group(group) => {
                if is_hidden(&group.name, &source_module_name, &all_hidden_items) {
                    let err = ResolverError::ExplicitlyImportedTypeHidden(
                        group.name.clone(),
                        module.name.get(),
                        import.location_id,
                    );
                    errors.push(err);
                    continue;
                }
                match source_module.exported_items.get(&group.name) {
                    Some(item) => match item {
                        ExportedItem::Record(record_id) => {
                            import_exported_item(
                                &group.name,
                                item,
                                source_module,
                                imported_items,
                                namespace,
                                mode,
                            );
                            for member in &group.members {
                                match member {
                                    ImportedMember::All => {
                                        for (exported_member_name, source_items) in
                                            &source_module.exported_members
                                        {
                                            if is_hidden(
                                                exported_member_name,
                                                &source_module_name,
                                                &all_hidden_items,
                                            ) {
                                                continue;
                                            }
                                            for source_item in source_items {
                                                if let ExportedDataMember::RecordField(
                                                    exported_field,
                                                ) = source_item
                                                {
                                                    if exported_field.record_id == *record_id {
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
                                        if is_hidden(
                                            member_name,
                                            &source_module_name,
                                            &all_hidden_items,
                                        ) {
                                            let err =
                                                ResolverError::ExplicitlyImportedRecordFieldHidden(
                                                    member_name.clone(),
                                                    module.name.get(),
                                                    import.location_id,
                                                );
                                            errors.push(err);
                                            continue;
                                        }
                                        match source_module.exported_members.get(member_name) {
                                            Some(source_items) => {
                                                let mut found = false;
                                                for source_item in source_items {
                                                    if let ExportedDataMember::RecordField(
                                                        exported_field,
                                                    ) = source_item
                                                    {
                                                        if exported_field.record_id == *record_id {
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
                                                    let err =
                                                    ResolverError::ImportedRecordFieldNotExported(
                                                        group.name.clone(),
                                                        member_name.clone(),
                                                        import.location_id,
                                                    );
                                                    errors.push(err);
                                                }
                                            }
                                            None => {
                                                let err =
                                                    ResolverError::ImportedRecordFieldNotExported(
                                                        group.name.clone(),
                                                        member_name.clone(),
                                                        import.location_id,
                                                    );
                                                errors.push(err);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        ExportedItem::Adt(adt_id) => {
                            import_exported_item(
                                &group.name,
                                item,
                                source_module,
                                imported_items,
                                namespace,
                                mode,
                            );
                        }
                        ExportedItem::Function(_) => {
                            let err = ResolverError::IncorrectNameInImportedTypeConstructor(
                                import.module_path.get(),
                                group.name.clone(),
                                import.location_id,
                            );
                            errors.push(err);
                        }
                    },
                    None => {
                        let err = ResolverError::IncorrectNameInImportedTypeConstructor(
                            import.module_path.get(),
                            group.name.clone(),
                            import.location_id,
                        );
                        errors.push(err);
                    }
                }
            }
            AstImportedItem::NamedItem(item_name) => {
                if is_hidden(item_name, &source_module_name, &all_hidden_items) {
                    let err = ResolverError::ExplicitlyImportedItemHidden(
                        item_name.clone(),
                        module.name.get(),
                        import.location_id,
                    );
                    errors.push(err);
                    continue;
                }

                match source_module.exported_items.get(item_name) {
                    Some(exported_item) => {
                        let imported_item = get_imported_item(exported_item);
                        let imported_item_info =
                            get_imported_item_info(imported_item, &source_module.name);
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
                            import.module_path.get(),
                            import.location_id,
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
    exported_member: &ExportedDataMember,
    source_module: &Module,
    imported_members: &mut BTreeMap<String, Vec<ImportedMemberInfo>>,
    namespace: &str,
    mode: ImportMode,
) {
    let (imported_member, is_record) = get_imported_member(exported_member);
    let imported_member_info = get_imported_member_info(imported_member, &source_module.name);
    let names = if is_record {
        vec![member_name.to_string()]
    } else {
        get_names(&namespace, member_name, mode)
    };
    for name in names {
        let imported_member_infos = imported_members
            .entry(name.clone())
            .or_insert_with(|| Vec::new());
        imported_member_infos.push(imported_member_info.clone());
    }
}

fn import_exported_item(
    item_name: &str,
    exported_item: &ExportedItem,
    source_module: &Module,
    imported_items: &mut BTreeMap<String, Vec<ImportedItemInfo>>,
    namespace: &str,
    mode: ImportMode,
) {
    let imported_item = get_imported_item(exported_item);
    let imported_item_info = get_imported_item_info(imported_item, &source_module.name);
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
    let source_module_name = source_module.name.get();
    for (item_name, exported_item) in &source_module.exported_items {
        if is_hidden(item_name, &source_module_name, &all_hidden_items) {
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
        if is_hidden(member_name, &source_module_name, &all_hidden_items) {
            continue;
        }
        for exported_member in exported_members {
            import_exported_member(
                member_name,
                exported_member,
                source_module,
                imported_members,
                namespace,
                mode,
            );
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
        println!("Processing imports for module {}", module_name);
        let mut all_hidden_items = BTreeMap::new();
        let mut imported_items = BTreeMap::new();
        let mut imported_members = BTreeMap::new();
        let ast_module = program.modules.get(&module.id).expect("Module not found");
        for (_, import) in &ast_module.imports {
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

        for (_, import) in &ast_module.imports {
            let source_module = match modules.get(&import.module_path.get()) {
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
                        None => (import.module_path.get(), ImportMode::NameAndNamespace),
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
