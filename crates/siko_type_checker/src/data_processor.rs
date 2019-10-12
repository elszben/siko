use crate::common::AdtTypeInfo;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::type_processor::process_type_signature;
use crate::type_store::TypeStore;
use crate::types::Type;
use siko_ir::program::Program;
use siko_ir::types::TypeDef;
use siko_ir::types::TypeDefId;
use std::collections::BTreeMap;

pub struct DataProcessor<'a> {
    program: &'a Program,
    type_store: &'a mut TypeStore,
    record_type_info_map: BTreeMap<TypeDefId, RecordTypeInfo>,
    adt_type_info_map: BTreeMap<TypeDefId, AdtTypeInfo>,
    variant_type_info_map: BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
}

impl<'a> DataProcessor<'a> {
    pub fn new(program: &'a Program, type_store: &'a mut TypeStore) -> DataProcessor<'a> {
        DataProcessor {
            program: program,
            type_store: type_store,
            record_type_info_map: BTreeMap::new(),
            adt_type_info_map: BTreeMap::new(),
            variant_type_info_map: BTreeMap::new(),
        }
    }

    pub fn process_data_typedefs(
        mut self,
    ) -> (
        BTreeMap<TypeDefId, RecordTypeInfo>,
        BTreeMap<TypeDefId, AdtTypeInfo>,
        BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    ) {
        for (id, typedef) in &self.program.typedefs.items {
            match typedef {
                TypeDef::Adt(adt) => {
                    let mut arg_map = BTreeMap::new();
                    let mut type_args: Vec<_> = Vec::new();
                    for arg in &adt.type_args {
                        let var = self.type_store.get_new_type_var();
                        arg_map.insert(*arg, var);
                        type_args.push(var);
                    }

                    let type_arg_vars: Vec<_> = arg_map.iter().map(|(_, var)| *var).collect();

                    let adt_type = Type::Named(adt.name.clone(), *id, type_args.clone());
                    let adt_type_var = self.type_store.add_type(adt_type);

                    let adt_type_info = AdtTypeInfo {
                        adt_type: adt_type_var,
                        type_arg_vars: type_arg_vars,
                    };
                    self.adt_type_info_map.insert(*id, adt_type_info);
                    for (variant_index, variant) in adt.variants.iter().enumerate() {
                        let mut item_types = Vec::new();
                        for item in &variant.items {
                            let item_var = process_type_signature(
                                &mut self.type_store,
                                &item.type_signature_id,
                                self.program,
                                &mut arg_map,
                                &mut None,
                            );
                            let location_id = self
                                .program
                                .type_signatures
                                .get(&item.type_signature_id)
                                .location_id;
                            item_types.push((item_var, location_id));
                        }
                        let adt_type = Type::Named(adt.name.clone(), *id, type_args.clone());
                        let adt_type_var = self.type_store.add_type(adt_type);

                        let location_id = self
                            .program
                            .type_signatures
                            .get(&variant.type_signature_id)
                            .location_id;

                        let variant_type_info = VariantTypeInfo {
                            variant_type: adt_type_var,
                            item_types: item_types,
                            location_id: location_id,
                        };

                        self.variant_type_info_map
                            .insert((*id, variant_index), variant_type_info);
                    }
                }
                TypeDef::Record(record) => {
                    let mut arg_map = BTreeMap::new();
                    let mut type_args: Vec<_> = Vec::new();
                    for arg in &record.type_args {
                        let var = self.type_store.get_new_type_var();
                        arg_map.insert(*arg, var);
                        type_args.push(var);
                    }

                    let type_arg_vars: Vec<_> = arg_map.iter().map(|(_, var)| *var).collect();

                    let mut field_types = Vec::new();
                    for field in &record.fields {
                        let field_var = process_type_signature(
                            &mut self.type_store,
                            &field.type_signature_id,
                            self.program,
                            &mut arg_map,
                            &mut None,
                        );
                        let location_id = self
                            .program
                            .type_signatures
                            .get(&field.type_signature_id)
                            .location_id;
                        field_types.push((field_var, location_id));
                    }
                    let record_type = Type::Named(record.name.clone(), *id, type_args);
                    let record_type_var = self.type_store.add_type(record_type);
                    let record_type_info = RecordTypeInfo {
                        record_type: record_type_var,
                        field_types: field_types,
                        type_arg_vars: type_arg_vars,
                    };
                    self.record_type_info_map
                        .insert(record.id, record_type_info);
                }
            }
        }

        (
            self.record_type_info_map,
            self.adt_type_info_map,
            self.variant_type_info_map,
        )
    }
}
