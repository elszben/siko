use crate::common::AdtTypeInfo;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::error::TypecheckError;
use crate::instance_resolver::ResolutionResult;
use crate::type_store::TypeStore;
use crate::type_variable::TypeVariable;
use crate::types::Type;
use siko_ir::class::ClassId;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_ir::types::Adt;
use siko_ir::types::AutoDeriveMode;
use siko_ir::types::Record;
use siko_ir::types::TypeDef;
use siko_ir::types::TypeDefId;
use siko_ir::walker::Visitor;
use siko_location_info::item::LocationId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::dependency_processor::DependencyCollector;
use crate::dependency_processor::DependencyGroup;
use crate::dependency_processor::DependencyProcessor;

pub enum AutoDeriveState {
    No,
    Yes(Vec<Vec<ClassId>>),
}

pub struct Item {
    typedef_id: TypeDefId,
    state: AutoDeriveState,
    dependencies: Vec<TypeDefId>,
}

struct FunctionDependencyCollector<'a> {
    program: &'a Program,
    used_functions: BTreeSet<FunctionId>,
}

impl<'a> FunctionDependencyCollector<'a> {
    fn new(program: &'a Program) -> FunctionDependencyCollector<'a> {
        FunctionDependencyCollector {
            program: program,
            used_functions: BTreeSet::new(),
        }
    }
}

impl<'a> Visitor for FunctionDependencyCollector<'a> {
    fn get_program(&self) -> &Program {
        &self.program
    }

    fn visit_expr(&mut self, _: ExprId, expr: &Expr) {
        match expr {
            Expr::StaticFunctionCall(id, _) => {
                self.used_functions.insert(*id);
            }
            _ => {}
        }
    }

    fn visit_pattern(&mut self, _: PatternId, _: &Pattern) {
        // do nothing
    }
}

fn check_type_arg_vars(
    derived_class: ClassId,
    name: &str,
    location_id: LocationId,
    type_arg_vars: &Vec<TypeVariable>,
    errors: &mut Vec<TypecheckError>,
    type_store: &mut TypeStore,
    program: &Program,
) -> bool {
    for type_arg_var in type_arg_vars {
        let ty = type_store.get_type(type_arg_var);
        match ty {
            Type::TypeArgument(_, _) => {}
            Type::FixedTypeArgument(_, _, _) => {}
            _ => {
                let class = program.classes.get(&derived_class);
                let err = TypecheckError::AutoDeriveMemberInstanceNotGeneric(
                    name.to_string(),
                    location_id,
                    class.name.clone(),
                );
                errors.push(err);
                return false;
            }
        }
    }
    return true;
}

fn check_data_type(
    type_arg_vars: Vec<TypeVariable>,
    data_type_var: TypeVariable,
    typedef_id: &TypeDefId,
    name: String,
    derived_class: ClassId,
    members: Vec<(TypeVariable, LocationId)>,
    errors: &mut Vec<TypecheckError>,
    derive_location: Option<LocationId>,
    type_store: &mut TypeStore,
    program: &Program,
) -> Item {
    let mut dependencies = Vec::new();
    let resolution_result = type_store.has_class_instance(&data_type_var, &derived_class);
    let state = {
        match resolution_result {
            ResolutionResult::Definite(instance_id) => {
                if let Some(derive_location) = derive_location {
                    let instance = program.instances.get(&instance_id);
                    let class = program.classes.get(&instance.class_id);
                    let err = TypecheckError::AutoDeriveConflict(
                        name.clone(),
                        derive_location,
                        instance.location_id,
                        class.name.clone(),
                    );
                    errors.push(err);
                }
                AutoDeriveState::Yes(Vec::new())
            }
            ResolutionResult::Inconclusive => unreachable!(),
            ResolutionResult::No => {
                let mut first_non_generic_instance = true;
                let mut member_failed = false;
                for member in &members {
                    let ty = type_store.get_type(&member.0);
                    match ty {
                        Type::TypeArgument(_, _) => {}
                        Type::FixedTypeArgument(_, _, _) => {}
                        Type::Named(_, dep_id, _) => {
                            if dep_id == *typedef_id {
                                continue;
                            }
                            let resolution_result =
                                type_store.has_class_instance(&member.0, &derived_class);
                            if let ResolutionResult::Definite(_) = resolution_result {
                                //println!("{:?}", resolution_result);
                                if first_non_generic_instance {
                                    if !check_type_arg_vars(
                                        derived_class,
                                        &name,
                                        member.1,
                                        &type_arg_vars,
                                        errors,
                                        type_store,
                                        program,
                                    ) {
                                        member_failed = true;
                                        first_non_generic_instance = false;
                                    }
                                }
                            }
                            dependencies.push(dep_id);
                        }
                        _ => {
                            panic!("type as member is not yet implemented {:?}", ty);
                        }
                    }
                }
                if member_failed {
                    AutoDeriveState::No
                } else {
                    let mut constraints = Vec::new();
                    for type_arg_var in type_arg_vars {
                        let ty = type_store.get_type(&type_arg_var);
                        match ty {
                            Type::TypeArgument(_, arg_constraints) => {
                                constraints.push(arg_constraints.clone());
                            }
                            _ => {
                                panic!("Type arg var is not a type argument but member_failed is not set for it");
                            }
                        }
                    }
                    AutoDeriveState::Yes(constraints)
                }
            }
        }
    };
    Item {
        typedef_id: *typedef_id,
        state: state,
        dependencies: dependencies,
    }
}

fn check_adt(
    adt: &Adt,
    derived_class: ClassId,
    errors: &mut Vec<TypecheckError>,
    derive_location: Option<LocationId>,
    adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
    variant_type_info_map: &BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
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
        derive_location,
        type_store,
        program,
    )
}

fn check_record(
    record: &Record,
    derived_class: ClassId,
    errors: &mut Vec<TypecheckError>,
    derive_location: Option<LocationId>,
    record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    type_store: &mut TypeStore,
    program: &Program,
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
        derive_location,
        type_store,
        program,
    )
}

pub struct TypedefDependencyProcessor<'a> {
    program: &'a Program,
    type_store: &'a mut TypeStore,
    adt_type_info_map: &'a BTreeMap<TypeDefId, AdtTypeInfo>,
    record_type_info_map: &'a BTreeMap<TypeDefId, RecordTypeInfo>,
    variant_type_info_map: &'a BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    class_items_map: BTreeMap<ClassId, Vec<Item>>,
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
            .or_insert_with(|| Vec::new());
        items.push(item)
    }

    pub fn process_functions(
        &mut self,
        errors: &mut Vec<TypecheckError>,
    ) -> Vec<DependencyGroup<TypeDefId>> {
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
                                None,
                                self.adt_type_info_map,
                                self.variant_type_info_map,
                                self.type_store,
                                self.program,
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
                                Some(derived_class.location_id),
                                self.adt_type_info_map,
                                self.variant_type_info_map,
                                self.type_store,
                                self.program,
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
                                None,
                                self.record_type_info_map,
                                self.type_store,
                                self.program,
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
                                Some(derived_class.location_id),
                                self.record_type_info_map,
                                self.type_store,
                                self.program,
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
            for item in items.iter() {
                current_processed_typedefs.insert(item.typedef_id);
            }
            for item in items.iter() {
                for dep in &item.dependencies {
                    if !current_processed_typedefs.contains(dep) {
                        unprocesed_typedefs.insert(*dep);
                    }
                }
            }
            for typedef_id in unprocesed_typedefs {
                let typedef = self.program.typedefs.get(&typedef_id);
                match typedef {
                    TypeDef::Adt(adt) => {
                        let item = check_adt(
                            &adt,
                            *class_id,
                            errors,
                            None,
                            self.adt_type_info_map,
                            self.variant_type_info_map,
                            self.type_store,
                            self.program,
                        );
                        items.push(item);
                    }
                    TypeDef::Record(record) => {
                        let item = check_record(
                            &record,
                            *class_id,
                            errors,
                            None,
                            self.record_type_info_map,
                            self.type_store,
                            self.program,
                        );
                        items.push(item);
                    }
                }
            }
        }

        for (class_id, items) in &self.class_items_map {
            for item in items {
                let class = self.program.classes.get(&class_id);
                let (module, name) = self.program.get_module_and_name(item.typedef_id);
                let mut dependencies = Vec::new();
                for dep in &item.dependencies {
                    let (module, name) = self.program.get_module_and_name(*dep);
                    dependencies.push(format!("{}/{}", module, name));
                }
                println!(
                    "class {}, type {}/{}, dependencies ({})",
                    class.name,
                    module,
                    name,
                    dependencies.join(", ")
                );
            }
            let typedef_ids: Vec<_> = items.iter().map(|item| item.typedef_id).collect();
            let dep_processor = DependencyProcessor::new(typedef_ids);
            let ordered_function_groups = dep_processor.process_items(self);
        }
        Vec::new()
    }
}

impl<'a> DependencyCollector<TypeDefId> for TypedefDependencyProcessor<'a> {
    fn collect(&self, typedef_id: TypeDefId) -> Vec<TypeDefId> {
        let mut deps = BTreeSet::new();
        deps.into_iter().collect()
    }
}
