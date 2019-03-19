use crate::name_resolution::error::ResolverError;
use crate::name_resolution::export::ExportedField;
use crate::name_resolution::export::ExportedItem;
use crate::name_resolution::export::ExportedType;
use crate::name_resolution::export::ExportedVariant;
use crate::name_resolution::item::Item;
use crate::name_resolution::item::Type;
use crate::name_resolution::module::Module;
use crate::syntax::data::AdtId;
use crate::syntax::data::RecordId;
use crate::syntax::export::ExportList;
use crate::syntax::export::ExportedDataConstructor;
use crate::syntax::export::ExportedItem as AstExportedItem;
use crate::syntax::program::Program;
use std::collections::BTreeMap;

pub fn process_exports(
    modules: &mut BTreeMap<String, Module>,
    program: &Program,
    errors: &mut Vec<ResolverError>,
) {
    for (module_name, module) in modules.iter_mut() {
        let mut exported_items = BTreeMap::new();
        let mut exported_types = BTreeMap::new();
        let mut exported_fields = BTreeMap::new();
        let mut exported_variants = BTreeMap::new();
        let ast_module = program.modules.get(&module.id).expect("Module not found");
        match &ast_module.export_list {
            ExportList::ImplicitAll => {
                for (name, items) in &module.items {
                    assert_eq!(items.len(), 1);
                    let item = &items[0];
                    match item {
                        Item::Function(function_id) => {
                            exported_items
                                .insert(name.clone(), ExportedItem::Function(*function_id));
                        }
                        Item::Record(record_id) => {
                            exported_items.insert(name.clone(), ExportedItem::Record(*record_id));
                            export_all_fields(record_id, program, &mut exported_fields);
                        }
                        Item::DataConstructor(_) => {}
                    }
                }
                for (name, types) in &module.types {
                    assert_eq!(types.len(), 1);
                    let ty = &types[0];
                    match ty {
                        Type::Record(record_id) => {
                            exported_types.insert(name.clone(), ExportedType::Record(*record_id));
                        }
                        Type::Adt(adt_id) => {
                            exported_types.insert(name.clone(), ExportedType::Adt(*adt_id));
                            export_all_variants(adt_id, program, &mut exported_variants);
                        }
                    }
                }
            }
            ExportList::Explicit(items) => {
                for item in items {
                    match item {
                        AstExportedItem::Named(entity_name) => {
                            let mut found = false;
                            match module.items.get(entity_name) {
                                Some(items) => {
                                    assert_eq!(items.len(), 1);
                                    let item = &items[0];
                                    match item {
                                        Item::Function(function_id) => {
                                            found = true;
                                            exported_items.insert(
                                                entity_name.clone(),
                                                ExportedItem::Function(*function_id),
                                            );
                                        }
                                        Item::DataConstructor(_) => {
                                            // cannot export a data constructor as a stand alone export list item
                                            // this is ignored
                                        }
                                        Item::Record(record_id) => {
                                            found = true;
                                            exported_items.insert(
                                                entity_name.clone(),
                                                ExportedItem::Record(*record_id),
                                            );
                                        }
                                    }
                                }
                                None => {}
                            }
                            match module.types.get(entity_name) {
                                Some(items) => {
                                    found = true;
                                    assert_eq!(items.len(), 1);
                                    let item = &items[0];
                                    match item {
                                        Type::Record(record_id) => {
                                            exported_types.insert(
                                                entity_name.clone(),
                                                ExportedType::Record(*record_id),
                                            );
                                        }
                                        Type::Adt(adt_id) => {
                                            exported_types.insert(
                                                entity_name.clone(),
                                                ExportedType::Adt(*adt_id),
                                            );
                                        }
                                    }
                                }
                                None => {}
                            }
                            if !found {
                                let err = ResolverError::ExportedEntityDoesNotExist(
                                    module_name.clone(),
                                    entity_name.clone(),
                                    ast_module.location_id,
                                );
                                errors.push(err);
                            }
                        }
                        AstExportedItem::Adt(type_ctor) => {
                            match module.types.get(&type_ctor.name) {
                                Some(types) => {
                                    assert_eq!(types.len(), 1);
                                    let ty = &types[0];
                                    match ty {
                                        Type::Record(record_id) => {
                                            for data_ctor in &type_ctor.data_constructors {
                                                match data_ctor {
                                                    ExportedDataConstructor::All => {
                                                        export_all_fields(
                                                            record_id,
                                                            program,
                                                            &mut exported_fields,
                                                        );
                                                    }
                                                    ExportedDataConstructor::Specific(
                                                        field_name,
                                                    ) => {
                                                        let record = program
                                                            .records
                                                            .get(record_id)
                                                            .expect("Record not found");
                                                        let mut found = false;
                                                        for field in &record.fields {
                                                            if &field.name == field_name {
                                                                found = true;
                                                                let fields = exported_fields
                                                                    .entry(field.name.clone())
                                                                    .or_insert_with(|| Vec::new());
                                                                let exported_field =
                                                                    ExportedField {
                                                                        field_id: field.id,
                                                                        record_id: record.id,
                                                                    };
                                                                fields.push(exported_field);
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
                                        Type::Adt(adt_id) => {
                                            for data_ctor in &type_ctor.data_constructors {
                                                match data_ctor {
                                                    ExportedDataConstructor::All => {
                                                        export_all_variants(
                                                            adt_id,
                                                            program,
                                                            &mut exported_variants,
                                                        );
                                                    }
                                                    ExportedDataConstructor::Specific(
                                                        variant_name,
                                                    ) => {
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
                                                                let variants = exported_variants
                                                                    .entry(variant.name.clone())
                                                                    .or_insert_with(|| Vec::new());
                                                                let exported_variant =
                                                                    ExportedVariant {
                                                                        variant_id: variant.id,
                                                                        adt_id: adt.id,
                                                                    };
                                                                variants.push(exported_variant);
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
                                    }
                                }
                                None => {
                                    let err = ResolverError::IncorrectNameInExportedTypeConstructor(
                                        module_name.clone(),
                                        type_ctor.name.clone(),
                                        module.location_id,
                                    );
                                    errors.push(err);
                                }
                            }
                        }
                    }
                }
            }
        }

        module.exported_items = exported_items;
        module.exported_types = exported_types;
        module.exported_fields = exported_fields;
        module.exported_variants = exported_variants;

        /*
        println!("Module {} exports:", module_name);
        println!(
            "{} exported items {} exported types, {} exported fields, {} exported variants ",
            module.exported_items.len(),
            module.exported_types.len(),
            module.exported_fields.len(),
            module.exported_variants.len(),
        );
        for (name, export) in &module.exported_items {
            println!("Item: {} => {:?}", name, export);
        }
        for (name, export) in &module.exported_types {
            println!("Type: {} => {:?}", name, export);
        }
        for (name, export) in &module.exported_fields {
            println!("Field: {} => {:?}", name, export);
        }
        for (name, export) in &module.exported_variants {
            println!("Variant: {} => {:?}", name, export);
        }
        */
    }
}

fn export_all_fields(
    record_id: &RecordId,
    program: &Program,
    exported_fields: &mut BTreeMap<String, Vec<ExportedField>>,
) {
    let record = program.records.get(record_id).expect("Record not found");
    for field in &record.fields {
        let fields = exported_fields
            .entry(field.name.clone())
            .or_insert_with(|| Vec::new());
        let exported_field = ExportedField {
            field_id: field.id,
            record_id: record.id,
        };
        fields.push(exported_field);
    }
}

fn export_all_variants(
    adt_id: &AdtId,
    program: &Program,
    exported_variants: &mut BTreeMap<String, Vec<ExportedVariant>>,
) {
    let adt = program.adts.get(adt_id).expect("Adt not found");
    for variant_id in &adt.variants {
        let variant = program.variants.get(variant_id).expect("Variant not found");
        let variants = exported_variants
            .entry(variant.name.clone())
            .or_insert_with(|| Vec::new());
        let exported_variant = ExportedVariant {
            variant_id: variant.id,
            adt_id: adt.id,
        };
        variants.push(exported_variant);
    }
}
