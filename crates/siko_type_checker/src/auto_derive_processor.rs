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

use crate::dependency_processor::DependencyCollector;
use crate::dependency_processor::DependencyProcessor;

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

#[derive(Debug, Eq, PartialEq)]
pub enum AutoDeriveState {
    No,
    Possible(Vec<Vec<ClassId>>),
}

pub struct Item {
    typedef_id: TypeDefId,
    state: AutoDeriveState,
    dependencies: Vec<(TypeDefId, LocationId)>,
}

impl Item {
    pub fn dump(&self, program: &Program, class_id: ClassId) {
        let class = program.classes.get(&class_id);
        let (module, name) = program.get_module_and_name(self.typedef_id);
        let mut dependencies = Vec::new();
        for dep in &self.dependencies {
            let (module, name) = program.get_module_and_name(dep.0);
            dependencies.push(format!("{}/{}", module, name));
        }
        println!(
            "class {}, type {}/{}, dependencies ({}) state {:?}",
            class.name,
            module,
            name,
            dependencies.join(", "),
            self.state
        );
    }
}

fn get_type_arg_constraints(
    derived_class: ClassId,
    name: &str,
    location_id: LocationId,
    type_arg_vars: &Vec<TypeVariable>,
    type_store: &TypeStore,
    errors: &mut Vec<TypecheckError>,
    program: &Program,
) -> Vec<Vec<ClassId>> {
    let mut constraints = Vec::new();
    for type_arg_var in type_arg_vars {
        let ty = type_store.get_type(&type_arg_var);
        match ty {
            Type::TypeArgument(_, arg_constraints) => {
                constraints.push(arg_constraints.clone());
            }
            _ => {
                let class = program.classes.get(&derived_class);
                let err = TypecheckError::AutoDeriveMemberInstanceNotGeneric(
                    name.to_string(),
                    location_id,
                    class.name.clone(),
                );
                errors.push(err);
                return Vec::new();
            }
        }
    }
    constraints
}

#[derive(Eq, PartialEq)]
enum CheckMode {
    ExplicitDerive(LocationId),
    ImplicitDerive,
    InstanceOnly(LocationId),
}

fn check_data_type(
    type_arg_vars: Vec<TypeVariable>,
    data_type_var: TypeVariable,
    typedef_id: &TypeDefId,
    name: String,
    derived_class: ClassId,
    members: Vec<(TypeVariable, LocationId)>,
    errors: &mut Vec<TypecheckError>,
    type_store: &mut TypeStore,
    program: &Program,
    check_mode: CheckMode,
) -> Item {
    let mut dependencies = BTreeMap::new();
    let resolution_result = type_store.has_class_instance(&data_type_var, &derived_class);
    let state = {
        match resolution_result {
            ResolutionResult::Definite(instance_id) => {
                if let CheckMode::ExplicitDerive(derive_location) = check_mode {
                    let instance = program.instances.get(&instance_id);
                    let class = program.classes.get(&instance.class_id);
                    let err = TypecheckError::AutoDeriveConflict(
                        name.clone(),
                        derive_location,
                        instance.location_id,
                        class.name.clone(),
                    );
                    errors.push(err);
                    AutoDeriveState::No
                } else {
                    if let CheckMode::InstanceOnly(location_id) = check_mode {
                        let constraints = get_type_arg_constraints(
                            derived_class,
                            &name,
                            location_id,
                            &type_arg_vars,
                            type_store,
                            errors,
                            program,
                        );
                        AutoDeriveState::Possible(constraints)
                    } else {
                        AutoDeriveState::Possible(Vec::new())
                    }
                }
            }
            ResolutionResult::Inconclusive => unreachable!(),
            ResolutionResult::No => match check_mode {
                CheckMode::InstanceOnly(_) => AutoDeriveState::No,
                _ => {
                    for member in &members {
                        let ty = type_store.get_type(&member.0);
                        match ty {
                            Type::TypeArgument(_, _) => {}
                            Type::FixedTypeArgument(_, _, _) => {}
                            Type::Named(_, dep_id, _) => {
                                if dep_id == *typedef_id {
                                    continue;
                                }
                                // will overwrite previous ones
                                dependencies.insert(dep_id, member.1);
                            }
                            _ => {
                                panic!("type as member is not yet implemented {:?}", ty);
                            }
                        }
                    }
                    AutoDeriveState::Possible(Vec::new())
                }
            },
        }
    };
    Item {
        typedef_id: *typedef_id,
        state: state,
        dependencies: dependencies.into_iter().collect(),
    }
}

fn check_adt(
    adt: &Adt,
    derived_class: ClassId,
    errors: &mut Vec<TypecheckError>,
    adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
    variant_type_info_map: &BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
    check_mode: CheckMode,
) -> Item {
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
    check_data_type(
        type_arg_vars,
        adt_type_var,
        &adt.id,
        adt.name.clone(),
        derived_class,
        members,
        errors,
        type_store,
        program,
        check_mode,
    )
}

fn check_record(
    record: &Record,
    derived_class: ClassId,
    errors: &mut Vec<TypecheckError>,
    record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
    check_mode: CheckMode,
) -> Item {
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
    check_data_type(
        type_arg_vars,
        record_type_var,
        &record.id,
        record.name.clone(),
        derived_class,
        members,
        errors,
        type_store,
        program,
        check_mode,
    )
}

pub struct TypedefDependencyProcessor<'a> {
    program: &'a Program,
    type_store: &'a mut TypeStore,
    adt_type_info_map: &'a BTreeMap<TypeDefId, AdtTypeInfo>,
    record_type_info_map: &'a BTreeMap<TypeDefId, RecordTypeInfo>,
    variant_type_info_map: &'a BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    class_items_map: BTreeMap<ClassId, BTreeMap<TypeDefId, Item>>,
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
            class_items_map: BTreeMap::new(),
        }
    }

    fn add_item(&mut self, class_id: ClassId, item: Item) {
        let items = self
            .class_items_map
            .entry(class_id)
            .or_insert_with(|| BTreeMap::new());
        let id = item.typedef_id;
        items.insert(id, item);
    }

    pub fn process(&mut self, errors: &mut Vec<TypecheckError>) {
        //let implicit_derived_classes = [ClassId { id: 0 }];
        let implicit_derived_classes = [];
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
                            let item = check_adt(
                                adt,
                                *derived_class,
                                errors,
                                self.adt_type_info_map,
                                self.variant_type_info_map,
                                self.type_store,
                                self.program,
                                CheckMode::ImplicitDerive,
                            );
                            self.add_item(*derived_class, item);
                        }
                    }
                    AutoDeriveMode::Explicit(derived_classes) => {
                        for derived_class in derived_classes {
                            let item = check_adt(
                                adt,
                                derived_class.class_id,
                                errors,
                                self.adt_type_info_map,
                                self.variant_type_info_map,
                                self.type_store,
                                self.program,
                                CheckMode::ExplicitDerive(derived_class.location_id),
                            );
                            self.add_item(derived_class.class_id, item);
                        }
                    }
                },
                TypeDef::Record(record) => match &record.auto_derive_mode {
                    AutoDeriveMode::Implicit => {
                        for derived_class in &implicit_derived_classes {
                            let item = check_record(
                                record,
                                *derived_class,
                                errors,
                                self.record_type_info_map,
                                self.type_store,
                                self.program,
                                CheckMode::ImplicitDerive,
                            );
                            self.add_item(*derived_class, item);
                        }
                    }
                    AutoDeriveMode::Explicit(derived_classes) => {
                        for derived_class in derived_classes {
                            let item = check_record(
                                record,
                                derived_class.class_id,
                                errors,
                                self.record_type_info_map,
                                self.type_store,
                                self.program,
                                CheckMode::ExplicitDerive(derived_class.location_id),
                            );
                            self.add_item(derived_class.class_id, item);
                        }
                    }
                },
            }
        }

        for (class_id, items) in &mut self.class_items_map {
            let mut current_processed_typedefs = BTreeSet::new();
            let mut unprocesed_typedefs = BTreeSet::new();
            for (_, item) in items.iter() {
                current_processed_typedefs.insert(item.typedef_id);
            }
            for (_, item) in items.iter() {
                for dep in &item.dependencies {
                    if !current_processed_typedefs.contains(&dep.0) {
                        unprocesed_typedefs.insert(*dep);
                    }
                }
            }
            for typedef_info in unprocesed_typedefs {
                let typedef = self.program.typedefs.get(&typedef_info.0);
                match typedef {
                    TypeDef::Adt(adt) => {
                        let item = check_adt(
                            &adt,
                            *class_id,
                            errors,
                            self.adt_type_info_map,
                            self.variant_type_info_map,
                            self.type_store,
                            self.program,
                            CheckMode::InstanceOnly(typedef_info.1),
                        );
                        items.insert(typedef_info.0, item);
                    }
                    TypeDef::Record(record) => {
                        let item = check_record(
                            &record,
                            *class_id,
                            errors,
                            self.record_type_info_map,
                            self.type_store,
                            self.program,
                            CheckMode::InstanceOnly(typedef_info.1),
                        );
                        items.insert(typedef_info.0, item);
                    }
                }
            }
        }

        for (class_id, items) in &mut self.class_items_map {
            let typedef_ids: Vec<_> = items.iter().map(|(_, item)| item.typedef_id).collect();
            let dep_processor = DependencyProcessor::new(typedef_ids);
            let collector = TypedefDependencyCollector { items: items };
            let ordered_function_groups = dep_processor.process_items(&collector);
            println!("{} groups found ", ordered_function_groups.len());
            for (index, group) in ordered_function_groups.iter().enumerate() {
                println!("{}. group", index);
                for id in &group.items {
                    let item = items.get(&id).expect("Item not found");
                    //item.dump(self.program, *class_id);
                    let mut dep_failed = false;
                    for dep in &item.dependencies {
                        let dep_item = items.get(&dep.0).expect("Item not found");
                        if dep_item.state == AutoDeriveState::No {
                            dep_failed = true;
                            break;
                        }
                    }
                    if dep_failed {
                        let item = items.get_mut(&id).expect("Item not found");
                        item.state = AutoDeriveState::No;
                    }
                }
                let mut group_failed = false;
                for id in &group.items {
                    let item = items.get(&id).expect("Item not found");
                    if item.state == AutoDeriveState::No {
                        group_failed = true;
                        break;
                    }
                }
                if group_failed {
                    for id in &group.items {
                        let item = items.get_mut(&id).expect("Item not found");
                        item.state = AutoDeriveState::No;
                    }
                }
                for id in &group.items {
                    let item = items.get(&id).expect("Item not found");
                    item.dump(self.program, *class_id);
                }
            }
        }
    }
}

struct TypedefDependencyCollector<'a> {
    items: &'a BTreeMap<TypeDefId, Item>,
}

impl<'a> DependencyCollector<TypeDefId> for TypedefDependencyCollector<'a> {
    fn collect(&self, typedef_id: TypeDefId) -> Vec<TypeDefId> {
        let item = self.items.get(&typedef_id).expect("Item not found");
        item.dependencies.iter().map(|(id, _)| *id).collect()
    }
}
