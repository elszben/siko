use crate::name_resolution::error::ResolverError;
use crate::name_resolution::export::ExportedDataMember;
use crate::name_resolution::export::ExportedField;
use crate::name_resolution::export::ExportedVariant;
use crate::name_resolution::item::Item;
use crate::name_resolution::module::Module;
use crate::syntax::data::AdtId;
use crate::syntax::data::RecordId;
use crate::syntax::export::ExportList;
use crate::syntax::export::ExportedItem as AstExportedItem;
use crate::syntax::export::ExportedMember;
use crate::syntax::program::Program;
use std::collections::BTreeMap;

pub fn process_exports(
    modules: &mut BTreeMap<String, Module>,
    program: &Program,
    errors: &mut Vec<ResolverError>,
) {
    for (module_name, module) in modules.iter_mut() {
        let mut exported_items = BTreeMap::new();
        let mut exported_members = BTreeMap::new();
        let ast_module = program.modules.get(&module.id).expect("Module not found");
        match &ast_module.export_list {
            ExportList::ImplicitAll => {
                for (name, items) in &module.items {
                    assert_eq!(items.len(), 1);
                    let item = &items[0];
                    exported_items.insert(name.clone(), item.clone());
                    match item {
                        Item::Function(..) => {}
                        Item::Record(record_id, ir_typedef_id) => {
                            export_all_fields(record_id, program, &mut exported_members);
                        }
                        Item::Adt(adt_id, ir_typedef_id) => {
                            export_all_variants(adt_id, program, &mut exported_members);
                        }
                    }
                }
            }
            ExportList::Explicit(items) => {
                for item in items {
                    match item {
                        AstExportedItem::Named(entity_name) => {
                            match module.items.get(entity_name) {
                                Some(items) => {
                                    assert_eq!(items.len(), 1);
                                    let item = &items[0];
                                    exported_items.insert(entity_name.clone(), item.clone());
                                }
                                None => {
                                    let err = ResolverError::ExportedEntityDoesNotExist(
                                        module_name.clone(),
                                        entity_name.clone(),
                                        ast_module.location_id,
                                    );
                                    errors.push(err);
                                }
                            }
                        }
                        AstExportedItem::Group(group) => match module.items.get(&group.name) {
                            Some(items) => {
                                assert_eq!(items.len(), 1);
                                let item = &items[0];
                                match item {
                                    Item::Record(record_id, ir_typedef_id) => {
                                        exported_items.insert(group.name.clone(), item.clone());
                                        for member in &group.members {
                                            match member {
                                                ExportedMember::All => {
                                                    export_all_fields(
                                                        record_id,
                                                        program,
                                                        &mut exported_members,
                                                    );
                                                }
                                                ExportedMember::Specific(field_name) => {
                                                    let record = program
                                                        .records
                                                        .get(record_id)
                                                        .expect("Record not found");
                                                    let mut found = false;
                                                    for field in &record.fields {
                                                        if &field.name == field_name {
                                                            found = true;
                                                            let members = exported_members
                                                                .entry(field.name.clone())
                                                                .or_insert_with(|| Vec::new());
                                                            let exported_field = ExportedField {
                                                                field_id: field.id,
                                                                record_id: record.id,
                                                            };
                                                            members.push(
                                                                ExportedDataMember::RecordField(
                                                                    exported_field,
                                                                ),
                                                            );
                                                            break;
                                                        }
                                                    }
                                                    if !found {
                                                        let err = ResolverError::ExportedRecordFieldDoesNotExist(record.name.clone(), field_name.clone(), module.location_id);
                                                        errors.push(err);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Item::Adt(adt_id, ir_typedef_id) => {
                                        exported_items.insert(group.name.clone(), item.clone());
                                        for member in &group.members {
                                            match member {
                                                ExportedMember::All => {
                                                    export_all_variants(
                                                        adt_id,
                                                        program,
                                                        &mut exported_members,
                                                    );
                                                }
                                                ExportedMember::Specific(variant_name) => {
                                                    let adt = program
                                                        .adts
                                                        .get(adt_id)
                                                        .expect("Adt not found");
                                                    let mut found = false;
                                                    for variant_id in &adt.variants {
                                                        let variant = program
                                                            .variants
                                                            .get(variant_id)
                                                            .expect("Variant not found");
                                                        if &variant.name == variant_name {
                                                            found = true;
                                                            let members = exported_members
                                                                .entry(variant.name.clone())
                                                                .or_insert_with(|| Vec::new());
                                                            let exported_variant =
                                                                ExportedVariant {
                                                                    variant_id: variant.id,
                                                                    adt_id: adt.id,
                                                                };
                                                            members.push(
                                                                ExportedDataMember::Variant(
                                                                    exported_variant,
                                                                ),
                                                            );
                                                            break;
                                                        }
                                                    }
                                                    if !found {
                                                        let err = ResolverError::ExportedAdtVariantDoesNotExist(adt.name.clone(), variant_name.clone(), module.location_id);
                                                        errors.push(err);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {
                                        let err =
                                            ResolverError::IncorrectNameInExportedTypeConstructor(
                                                module_name.clone(),
                                                group.name.clone(),
                                                module.location_id,
                                            );
                                        errors.push(err);
                                    }
                                }
                            }
                            None => {
                                let err = ResolverError::IncorrectNameInExportedTypeConstructor(
                                    module_name.clone(),
                                    group.name.clone(),
                                    module.location_id,
                                );
                                errors.push(err);
                            }
                        },
                    }
                }
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

fn export_all_fields(
    record_id: &RecordId,
    program: &Program,
    exported_members: &mut BTreeMap<String, Vec<ExportedDataMember>>,
) {
    let record = program.records.get(record_id).expect("Record not found");
    for field in &record.fields {
        let members = exported_members
            .entry(field.name.clone())
            .or_insert_with(|| Vec::new());
        let exported_field = ExportedField {
            field_id: field.id,
            record_id: record.id,
        };
        members.push(ExportedDataMember::RecordField(exported_field));
    }
}

fn export_all_variants(
    adt_id: &AdtId,
    program: &Program,
    exported_members: &mut BTreeMap<String, Vec<ExportedDataMember>>,
) {
    let adt = program.adts.get(adt_id).expect("Adt not found");
    for variant_id in &adt.variants {
        let variant = program.variants.get(variant_id).expect("Variant not found");
        let members = exported_members
            .entry(variant.name.clone())
            .or_insert_with(|| Vec::new());
        let exported_variant = ExportedVariant {
            variant_id: variant.id,
            adt_id: adt.id,
        };
        members.push(ExportedDataMember::Variant(exported_variant));
    }
}
