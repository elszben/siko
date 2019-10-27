use crate::common::AdtTypeInfo;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::error::TypecheckError;
use crate::instance_resolver::ResolutionResult;
use crate::type_store::ResolverContext;
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
    TypeClassConstraint(TypeVariable, TypeDefId, ClassId, usize, LocationId),
    TypeClassInstanceNeed(TypeVariable, ClassId, LocationId),
}

pub struct InstanceInfo {
    typedef_id: TypeDefId,
    class_id: ClassId,
    type_args: Vec<TypeVariable>,
}

impl InstanceInfo {
    pub fn new(
        typedef_id: TypeDefId,
        class_id: ClassId,
        type_args: Vec<TypeVariable>,
    ) -> InstanceInfo {
        InstanceInfo {
            typedef_id: typedef_id,
            class_id: class_id,
            type_args: type_args,
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
    location_id: LocationId,
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
            if is_member {
                let constraint = Constraint::TypeClassInstanceNeed(var, class, location_id);
                constraints.push(constraint);
            }
        }
        Type::FixedTypeArgument(_, _, _) => {}
        Type::Named(_, dep_id, args) => {
            let (module, name) = program.get_module_and_name(dep_id);
            //println!("{} {}/{} ", var, module, name);
            if is_member {
                let constraint = Constraint::TypeClassInstanceNeed(var, class, location_id);
                constraints.push(constraint);
            }
            for (index, arg) in args.iter().enumerate() {
                /*println!(
                    "{}. type arg {} must match the constraints of {}. type arg of {}/{}",
                    index, arg, index, module, name
                );*/
                let constraint =
                    Constraint::TypeClassConstraint(*arg, dep_id, class, index, location_id);
                constraints.push(constraint);
                process_type_var(
                    *arg,
                    location_id,
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
                    location_id,
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

fn get_adt_type_vars(
    adt_id: TypeDefId,
    adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
    type_store: &mut TypeStore,
) -> (TypeVariable, Vec<TypeVariable>) {
    let adt_type_info = adt_type_info_map
        .get(&adt_id)
        .expect("Adt type info not found");
    let mut clone_context = type_store.create_clone_context();
    let mut type_arg_vars = Vec::new();
    for type_arg_var in &adt_type_info.type_arg_vars {
        let var = clone_context.clone_var(*type_arg_var);
        type_arg_vars.push(var);
    }
    let adt_type_var = clone_context.clone_var(adt_type_info.adt_type);
    (adt_type_var, type_arg_vars)
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
    let mut classes = Vec::new();
    match &adt.auto_derive_mode {
        AutoDeriveMode::Implicit => {}
        AutoDeriveMode::Explicit(derived_classes) => {
            for derived_class in derived_classes {
                classes.push(derived_class.class_id);
            }
        }
    }

    let (module, name) = program.get_module_and_name(adt.id);

    for class in &classes {
        let adt_type_info = adt_type_info_map
            .get(&adt.id)
            .expect("Adt type info not found");
        let mut clone_context = type_store.create_clone_context();
        let mut type_arg_vars = Vec::new();
        for type_arg_var in &adt_type_info.type_arg_vars {
            let var = clone_context.clone_var(*type_arg_var);
            type_arg_vars.push(var);
        }
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
        let instance_info = InstanceInfo::new(adt.id, *class, type_arg_vars.clone());
        instances.insert((adt.id, *class), instance_info);
        for member in &members {
            process_type_var(
                member.0,
                member.1,
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
            /*println!(
                "I-> {}/{} ({}) {} {}, {}",
                module,
                name,
                id,
                class_id,
                class.name,
                instance_info.type_args.len()
            );*/
        }
        loop {
            let mut modified = false;
            for constraint in &constraints {
                match constraint {
                    Constraint::TypeClassInstanceNeed(var, class, location_id) => {
                        let ir_class = self.program.classes.get(&class);
                        let var_s = self.type_store.get_resolved_type_string(var);
                        println!("N: {}/{} has to implement {}", var, var_s, ir_class.name);
                    }
                    Constraint::TypeClassConstraint(
                        var,
                        typedef_id,
                        class,
                        arg_index,
                        location_id,
                    ) => {
                        let (module, name) = self.program.get_module_and_name(*typedef_id);
                        let ir_class = self.program.classes.get(&class);
                        let var_s = self.type_store.get_resolved_type_string(var);
                        println!(
                            "{}/{} matches {} instance of {}/{} ({}) {} {} ",
                            var, var_s, ir_class.name, module, name, typedef_id, class, arg_index
                        );
                        /*let instance_info =
                            instances.entry((*typedef_id, *class)).or_insert_with(|| {
                                let (adt_type_var, type_arg_vars) = get_adt_type_vars(
                                    *typedef_id,
                                    self.adt_type_info_map,
                                    self.type_store,
                                );
                                match self.type_store.has_class_instance(&adt_type_var, &class) {
                                    ResolutionResult::Definite(_) => {
                                        InstanceInfo::new(*typedef_id, *class, type_arg_vars)
                                    }
                                    _ => {
                                        let err = TypecheckError::NoInstanceFoundDuringAutoDerive(
                                            name.clone(),
                                            ir_class.name.clone(),
                                            *location_id,
                                        );
                                        errors.push(err);
                                        InstanceInfo::new(*typedef_id, *class, Vec::new())
                                    }
                                }
                            });
                        if !errors.is_empty() {
                            break;
                        }
                        let instance_arg_var = instance_info.type_args[*arg_index];
                        let prev_index = self.type_store.get_index(&var);
                        if !self.type_store.constrain_type(&var, &instance_arg_var) {
                            let found_type = self.type_store.get_resolved_type_string(&var);
                            let expected_type =
                                self.type_store.get_resolved_type_string(&instance_arg_var);
                            let err = TypecheckError::ConstraintFailureDuringAutoDerive(
                                expected_type,
                                found_type,
                                ir_class.name.clone(),
                                *location_id,
                            );
                            errors.push(err);
                            break;
                        } else {
                            let index = self.type_store.get_index(&var);
                            if prev_index != index {
                                modified = true;
                            }
                        }*/
                    }
                }
            }
            if !modified {
                break;
            }
        }

        for ((id, class_id), instance_info) in &instances {
            let (module, name) = self.program.get_module_and_name(*id);
            let class = self.program.classes.get(class_id);
            println!(
                "I-> {}/{} ({}) {} {}",
                module, name, id, class_id, class.name,
            );
            let mut context = ResolverContext::new();
            for (index, ty_arg) in instance_info.type_args.iter().enumerate() {
                let ty = self
                    .type_store
                    .get_resolved_type_string_with_context(ty_arg, &mut context);
                println!("  {}. {}", index, ty);
            }
        }
    }
}
