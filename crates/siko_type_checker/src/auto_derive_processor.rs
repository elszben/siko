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
use siko_util::Counter;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

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

enum Constraint {
    TypeClassConstraint(TypeVariable, TypeDefId, ClassId, usize),
}

pub struct InstanceInfo {
    auto_derived: bool,
    typedef_id: TypeDefId,
    class_id: ClassId,
    constraints: Vec<Vec<ClassId>>,
}

impl InstanceInfo {
    pub fn new(typedef_id: TypeDefId, class_id: ClassId, type_arg_count: usize) -> InstanceInfo {
        InstanceInfo {
            auto_derived: false,
            typedef_id: typedef_id,
            class_id: class_id,
            constraints: std::iter::repeat(Vec::new()).take(type_arg_count).collect(),
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
    adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
    variant_type_info_map: &BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
    class: ClassId,
    type_arg_vars: &Vec<TypeVariable>,
    is_member: bool,
    constraints: &mut Vec<Constraint>,
) {
    let ty = type_store.get_type(&var);
    match ty {
        Type::TypeArgument(_, _) => {
            for (index, arg) in type_arg_vars.iter().enumerate() {
                if *arg == var && is_member {
                    println!("{}. type arg must have instance for {}", index, class);
                }
            }
        }
        Type::FixedTypeArgument(_, _, _) => {}
        Type::Named(_, dep_id, args) => {
            //let (module, name) = program.get_module_and_name(dep_id);
            //println!("{} {}/{} ", var, module, name);
            for (index, arg) in args.iter().enumerate() {
                /*println!(
                    "{}. type arg {} must match the constraints of {}. type arg of {}/{}",
                    index, arg, index, module, name
                );*/
                let constraint = Constraint::TypeClassConstraint(*arg, dep_id, class, index);
                constraints.push(constraint);
                process_type_var(
                    *arg,
                    adt_type_info_map,
                    variant_type_info_map,
                    record_type_info_map,
                    type_store,
                    program,
                    class,
                    type_arg_vars,
                    false,
                    constraints,
                );
            }
        }
        Type::Tuple(items) => {
            //println!("{} - TUPLE", indent(level));
            for item in items {
                process_type_var(
                    item,
                    adt_type_info_map,
                    variant_type_info_map,
                    record_type_info_map,
                    type_store,
                    program,
                    class,
                    type_arg_vars,
                    false,
                    constraints,
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
    adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
    variant_type_info_map: &BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
    instances: &mut BTreeMap<(TypeDefId, ClassId), InstanceInfo>,
    constraints: &mut Vec<Constraint>,
) {
    let (module, name) = program.get_module_and_name(adt.id);
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
    let mut classes = Vec::new();
    match &adt.auto_derive_mode {
        AutoDeriveMode::Implicit => {}
        AutoDeriveMode::Explicit(derived_classes) => {
            for derived_class in derived_classes {
                classes.push(derived_class.class_id);
            }
        }
    }
    for class in &classes {
        let instance_info = InstanceInfo::new(adt.id, *class, adt.type_args.len());
        instances.insert((adt.id, *class), instance_info);
    }
    println!(
        "ADT {}/{} type: {}, {}",
        module,
        name,
        adt_type_var,
        type_store.get_resolved_type_string(&adt_type_var)
    );
    for class in &classes {
        for member in &members {
            process_type_var(
                member.0,
                adt_type_info_map,
                variant_type_info_map,
                record_type_info_map,
                type_store,
                program,
                *class,
                &type_arg_vars,
                true,
                constraints,
            );
        }
    }
}

fn check_record(
    record: &Record,
    adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
    variant_type_info_map: &BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
    instances: &mut BTreeMap<(TypeDefId, ClassId), InstanceInfo>,
    constraints: &mut Vec<Constraint>,
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
    adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
    variant_type_info_map: &BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
    instances: &mut BTreeMap<(TypeDefId, ClassId), InstanceInfo>,
    constraints: &mut Vec<Constraint>,
) {
    let typedef = program.typedefs.get(&typedef_id);
    match typedef {
        TypeDef::Adt(adt) => {
            check_adt(
                adt,
                adt_type_info_map,
                variant_type_info_map,
                record_type_info_map,
                type_store,
                program,
                instances,
                constraints,
            );
        }
        TypeDef::Record(record) => {
            check_record(
                record,
                adt_type_info_map,
                variant_type_info_map,
                record_type_info_map,
                type_store,
                program,
                instances,
                constraints,
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
        let mut instances = BTreeMap::new();
        let mut constraints = Vec::new();
        for (id, typedef) in &self.program.typedefs.items {
            if let TypeDef::Record(record) = typedef {
                if record.fields.is_empty() {
                    //external
                    continue;
                }
            }
            check_typedef(
                *id,
                self.adt_type_info_map,
                self.variant_type_info_map,
                self.record_type_info_map,
                self.type_store,
                self.program,
                &mut instances,
                &mut constraints,
            );
        }
        for ((id, class_id), instance_info) in &instances {
            let (module, name) = self.program.get_module_and_name(*id);
            let class = self.program.classes.get(class_id);
            println!(
                "{}/{} ({}) {} {}, {}",
                module,
                name,
                id,
                class_id,
                class.name,
                instance_info.constraints.len()
            );
        }
        for constraint in constraints {
            match constraint {
                Constraint::TypeClassConstraint(var, typedef_id, class, arg_index) => {
                    let (module, name) = self.program.get_module_and_name(typedef_id);
                    println!(
                        "{}/{} ({}) {} {} {}. arg",
                        module, name, typedef_id, var, class, arg_index
                    );
                }
            }
        }
    }
}
