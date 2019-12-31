use crate::type_processor::process_type;
use siko_ir::data::TypeDef as IrTypeDef;
use siko_ir::program::Program as IrProgram;
use siko_ir::types::Type as IrType;
use siko_mir::data::Adt as MirAdt;
use siko_mir::data::Record as MirRecord;
use siko_mir::data::RecordField as MirRecordField;
use siko_mir::data::TypeDef as MirTypeDef;
use siko_mir::data::TypeDefId as MirTypeDefId;
use siko_mir::data::Variant as MirVariant;
use siko_mir::program::Program as MirProgram;
use siko_mir::types::Type as MirType;
use std::collections::BTreeMap;

pub struct TypeDefStore {
    typedefs: BTreeMap<IrType, MirTypeDefId>,
}

impl TypeDefStore {
    pub fn new() -> TypeDefStore {
        TypeDefStore {
            typedefs: BTreeMap::new(),
        }
    }

    pub fn add_tuple(
        &mut self,
        ty: IrType,
        field_types: Vec<MirType>,
        mir_program: &mut MirProgram,
    ) -> (String, MirTypeDefId) {
        let mut newly_added = false;
        let mir_typedef_id = *self.typedefs.entry(ty.clone()).or_insert_with(|| {
            newly_added = true;
            mir_program.typedefs.get_id()
        });
        let name = format!("tuple#{}", mir_typedef_id.id);
        if newly_added {
            let mut fields = Vec::new();
            for (index, field_ty) in field_types.into_iter().enumerate() {
                let mir_field = MirRecordField {
                    name: format!("field#{}", index),
                    ty: field_ty,
                };
                fields.push(mir_field);
            }
            let mir_record = MirRecord {
                id: mir_typedef_id,
                module: format!("<generated>"),
                name: name.clone(),
                fields: fields,
                external: false,
            };
            let mir_typedef = MirTypeDef::Record(mir_record);
            mir_program.typedefs.add_item(mir_typedef_id, mir_typedef);
        }
        (name, mir_typedef_id)
    }

    pub fn add_type(
        &mut self,
        ty: IrType,
        ir_program: &IrProgram,
        mir_program: &mut MirProgram,
    ) -> MirTypeDefId {
        let mut newly_added = false;
        let mir_typedef_id = *self.typedefs.entry(ty.clone()).or_insert_with(|| {
            newly_added = true;
            mir_program.typedefs.get_id()
        });
        if newly_added {
            match &ty {
                IrType::Named(_, ir_typedef_id, _) => {
                    let ir_typdef = ir_program.typedefs.get(ir_typedef_id);
                    match ir_typdef {
                        IrTypeDef::Adt(_) => {
                            let mut adt_type_info = ir_program
                                .adt_type_info_map
                                .get(ir_typedef_id)
                                .expect("Adt type info not found")
                                .clone();
                            let mut unifier = ir_program.get_unifier();
                            let r = unifier.unify(&adt_type_info.adt_type, &ty);
                            assert!(r.is_ok());
                            adt_type_info.apply(&unifier);
                            let ir_adt = ir_program.typedefs.get(ir_typedef_id).get_adt();
                            let mut variants = Vec::new();
                            for (index, variant) in adt_type_info.variant_types.iter().enumerate() {
                                let mut mir_item_types = Vec::new();
                                for (item_ty, _) in &variant.item_types {
                                    let mir_item_ty =
                                        process_type(item_ty, self, ir_program, mir_program);
                                    mir_item_types.push(mir_item_ty);
                                }
                                let mir_variant = MirVariant {
                                    name: ir_adt.variants[index].name.clone(),
                                    items: mir_item_types,
                                };
                                variants.push(mir_variant);
                            }
                            let mir_adt = MirAdt {
                                id: mir_typedef_id,
                                module: ir_adt.module.clone(),
                                name: ir_adt.name.clone(),
                                variants: variants,
                            };
                            let mir_typedef = MirTypeDef::Adt(mir_adt);
                            mir_program.typedefs.add_item(mir_typedef_id, mir_typedef);
                        }
                        IrTypeDef::Record(_) => {
                            let mut record_type_info = ir_program
                                .record_type_info_map
                                .get(ir_typedef_id)
                                .expect("Record type info not found")
                                .clone();
                            let mut unifier = ir_program.get_unifier();
                            let r = unifier.unify(&record_type_info.record_type, &ty);
                            assert!(r.is_ok());
                            record_type_info.apply(&unifier);
                            let ir_record = ir_program.typedefs.get(ir_typedef_id).get_record();
                            let mut fields = Vec::new();
                            for (index, (field_ty, _)) in
                                record_type_info.field_types.iter().enumerate()
                            {
                                let mir_field_ty =
                                    process_type(field_ty, self, ir_program, mir_program);
                                let mir_field = MirRecordField {
                                    name: ir_record.fields[index].name.clone(),
                                    ty: mir_field_ty,
                                };
                                fields.push(mir_field);
                            }
                            let mir_record = MirRecord {
                                id: mir_typedef_id,
                                module: ir_record.module.clone(),
                                name: ir_record.name.clone(),
                                fields: fields,
                                external: ir_record.external,
                            };
                            let mir_typedef = MirTypeDef::Record(mir_record);
                            mir_program.typedefs.add_item(mir_typedef_id, mir_typedef);
                        }
                    }
                }
                _ => unreachable!(),
            };
        }
        mir_typedef_id
    }
}
