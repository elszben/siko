use crate::common::AdtTypeInfo;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::error::TypecheckError;
use crate::instance_resolver::ResolutionResult;
use crate::type_store::TypeStore;
use crate::type_variable::TypeVariable;
use crate::types::Type;
use siko_ir::class::ClassId;
use siko_ir::program::Program;
use siko_ir::types::Adt;
use siko_ir::types::AutoDeriveMode;
use siko_ir::types::Record;
use siko_ir::types::TypeDef;
use siko_ir::types::TypeDefId;
use siko_location_info::item::LocationId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

/*
 * Auto derive rules:
 *
 * - Explicit auto derive conflicts with manual instance
 * - Auto derived instance must be generic
 * - Types including closures cannot derive instances (there will be exceptions*)
 *
 * Auto derive strategy:
 *
 * - A type will be able to use the auto derive mechanism for a given class iff
 *   - the given class has no instance for the given type
 *      if there is a given class, the auto derive mechanism fails:
 *      - in case of explicit auto derive -> compilation error
 *      - in case of implicit auto derive -> auto derive skipped, instance already exists
 *   - all members have an instance for the given class
 *
 *  - A type's members can include the type itself or other types whose members include the first type, thus
 *    creating cyclic dependencies. To solve this, the checker must create groups and resolve the auto derive
 *    state in the groups.
 *
 */

fn check_type_variable(
    var: TypeVariable,
    data_type_var: TypeVariable,
    typedef_id: &TypeDefId,
    derived_class: ClassId,
    members: Vec<(TypeVariable, LocationId)>,
    adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
    variant_type_info_map: &BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
    all_types: &mut BTreeMap<TypeDefId, BTreeSet<ClassId>>,
    level: usize,
) {
    let ty = type_store.get_type(&var);
    match ty {
        Type::TypeArgument(_, _) => {}
        Type::FixedTypeArgument(_, _, _) => {}
        Type::Named(_, dep_id, args) => {
            check_typedef(
                dep_id,
                derived_class,
                adt_type_info_map,
                variant_type_info_map,
                record_type_info_map,
                type_store,
                program,
                all_types,
                level,
            );
        }
        _ => {
            panic!("type as member is not yet implemented {:?}", ty);
        }
    }
}

fn indent(level: usize) -> String {
    std::iter::repeat(" ")
        .take(level)
        .collect::<Vec<_>>()
        .join("")
}

fn process_type_var(
    var: TypeVariable,
    derived_class: ClassId,
    adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
    variant_type_info_map: &BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
    all_types: &mut BTreeMap<TypeDefId, BTreeSet<ClassId>>,
    level: usize,
) {
    let ty = type_store.get_type(&var);
    match ty {
        Type::TypeArgument(_, _) => {
            println!(
                "{} - {} {}",
                indent(level),
                var,
                type_store.get_resolved_type_string(&var)
            );
        }
        Type::FixedTypeArgument(_, _, _) => {}
        Type::Named(_, dep_id, args) => {
            let (module, name) = program.get_module_and_name(dep_id);
            println!("{} - {} {}/{} ", indent(level), var, module, name);
            for arg in args {
                process_type_var(
                    arg,
                    derived_class,
                    adt_type_info_map,
                    variant_type_info_map,
                    record_type_info_map,
                    type_store,
                    program,
                    all_types,
                    level + 3,
                );
            }
        }
        Type::Tuple(items) => {
            println!("{} - TUPLE", indent(level));
            for item in items {
                process_type_var(
                    item,
                    derived_class,
                    adt_type_info_map,
                    variant_type_info_map,
                    record_type_info_map,
                    type_store,
                    program,
                    all_types,
                    level + 3,
                );
            }
        }
        _ => {
            panic!("type as member is not yet implemented {:?}", ty);
        }
    }
}

fn check_adt(
    adt: &Adt,
    derived_class: ClassId,
    adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
    variant_type_info_map: &BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
    all_types: &mut BTreeMap<TypeDefId, BTreeSet<ClassId>>,
    level: usize,
) {
    let (module, name) = program.get_module_and_name(adt.id);
    let class = program.classes.get(&derived_class);
    let adt_type_info = adt_type_info_map
        .get(&adt.id)
        .expect("Adt type info not found");
    let mut clone_context = type_store.create_clone_context();
    let mut type_arg_vars = Vec::new();
    for type_arg_var in &adt_type_info.type_arg_vars {
        let var = clone_context.clone_var(*type_arg_var);
        type_arg_vars.push(var);
    }
    let adt_type_var = clone_context.clone_var(adt_type_info.adt_type);
    let mut members = Vec::new();
    for index in 0..adt.variants.len() {
        let key = (adt.id, index);
        let variant_type_info = variant_type_info_map
            .get(&key)
            .expect("Variant type info not found");
        for item_type in &variant_type_info.item_types {
            let item_var = clone_context.clone_var(item_type.0);
            members.push((item_var, item_type.1));
        }
    }
    let type_arg_vars_str: Vec<_> = type_arg_vars.iter().map(|t| format!("{}", t)).collect();
    println!(
        "{} - ADT {}/{} -> {} type: {}, type_args ({}) {}",
        indent(level),
        module,
        name,
        class.name,
        adt_type_var,
        type_arg_vars_str.join(", "),
        type_store.get_resolved_type_string(&adt_type_var)
    );
    for member in &members {
        process_type_var(
            member.0,
            derived_class,
            adt_type_info_map,
            variant_type_info_map,
            record_type_info_map,
            type_store,
            program,
            all_types,
            level + 3,
        );
    }
}

fn check_record(
    record: &Record,
    derived_class: ClassId,
    adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
    variant_type_info_map: &BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
    all_types: &mut BTreeMap<TypeDefId, BTreeSet<ClassId>>,
    level: usize,
) {
    let record_type_info = record_type_info_map
        .get(&record.id)
        .expect("Record type info not found");
    let mut clone_context = type_store.create_clone_context();
    let mut type_arg_vars = Vec::new();
    for type_arg_var in &record_type_info.type_arg_vars {
        let var = clone_context.clone_var(*type_arg_var);
        type_arg_vars.push(var);
    }
    let record_type_var = clone_context.clone_var(record_type_info.record_type);
    let mut members = Vec::new();
    for field_type in &record_type_info.field_types {
        let item_var = clone_context.clone_var(field_type.0);
        members.push((item_var, field_type.1));
    }
}

fn check_typedef(
    typedef_id: TypeDefId,
    derived_class: ClassId,
    adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
    variant_type_info_map: &BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
    all_types: &mut BTreeMap<TypeDefId, BTreeSet<ClassId>>,
    level: usize,
) {
    let typedef = program.typedefs.get(&typedef_id);
    match typedef {
        TypeDef::Adt(adt) => {
            check_adt(
                adt,
                derived_class,
                adt_type_info_map,
                variant_type_info_map,
                record_type_info_map,
                type_store,
                program,
                all_types,
                level,
            );
        }
        TypeDef::Record(record) => {
            check_record(
                record,
                derived_class,
                adt_type_info_map,
                variant_type_info_map,
                record_type_info_map,
                type_store,
                program,
                all_types,
                level,
            );
        }
    }
}

pub struct TypedefDependencyProcessor<'a> {
    program: &'a Program,
    type_store: &'a mut TypeStore,
    adt_type_info_map: &'a BTreeMap<TypeDefId, AdtTypeInfo>,
    record_type_info_map: &'a BTreeMap<TypeDefId, RecordTypeInfo>,
    variant_type_info_map: &'a BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
}

impl<'a> TypedefDependencyProcessor<'a> {
    pub fn new(
        program: &'a Program,
        type_store: &'a mut TypeStore,
        adt_type_info_map: &'a BTreeMap<TypeDefId, AdtTypeInfo>,
        record_type_info_map: &'a BTreeMap<TypeDefId, RecordTypeInfo>,
        variant_type_info_map: &'a BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    ) -> TypedefDependencyProcessor<'a> {
        TypedefDependencyProcessor {
            program: program,
            type_store: type_store,
            adt_type_info_map: adt_type_info_map,
            record_type_info_map: record_type_info_map,
            variant_type_info_map: variant_type_info_map,
        }
    }

    pub fn process(&mut self, errors: &mut Vec<TypecheckError>) {
        //let implicit_derived_classes = [ClassId { id: 0 }];
        let implicit_derived_classes = [];
        let mut all_types = BTreeMap::new();
        for (id, typedef) in &self.program.typedefs.items {
            if let TypeDef::Record(record) = typedef {
                if record.fields.is_empty() {
                    //external
                    continue;
                }
            }
            let typedef = self.program.typedefs.get(id);
            match typedef {
                TypeDef::Adt(adt) => match &adt.auto_derive_mode {
                    AutoDeriveMode::Implicit => {
                        for derived_class in &implicit_derived_classes {
                            check_adt(
                                adt,
                                *derived_class,
                                self.adt_type_info_map,
                                self.variant_type_info_map,
                                self.record_type_info_map,
                                self.type_store,
                                self.program,
                                &mut all_types,
                                0,
                            );
                        }
                    }
                    AutoDeriveMode::Explicit(derived_classes) => {
                        for derived_class in derived_classes {
                            check_adt(
                                adt,
                                derived_class.class_id,
                                self.adt_type_info_map,
                                self.variant_type_info_map,
                                self.record_type_info_map,
                                self.type_store,
                                self.program,
                                &mut all_types,
                                0,
                            );
                        }
                    }
                },
                TypeDef::Record(record) => match &record.auto_derive_mode {
                    AutoDeriveMode::Implicit => {
                        for derived_class in &implicit_derived_classes {
                            check_record(
                                record,
                                *derived_class,
                                self.adt_type_info_map,
                                self.variant_type_info_map,
                                self.record_type_info_map,
                                self.type_store,
                                self.program,
                                &mut all_types,
                                0,
                            );
                        }
                    }
                    AutoDeriveMode::Explicit(derived_classes) => {
                        for derived_class in derived_classes {
                            check_record(
                                record,
                                derived_class.class_id,
                                self.adt_type_info_map,
                                self.variant_type_info_map,
                                self.record_type_info_map,
                                self.type_store,
                                self.program,
                                &mut all_types,
                                0,
                            );
                        }
                    }
                },
            }
        }
    }
}
