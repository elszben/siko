use crate::common::AdtTypeInfo;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::error::TypecheckError;
use crate::instance_resolver::ResolutionResult;
use crate::type_processor::process_type_signature;
use crate::type_store::CloneContext;
use crate::type_store::ResolverContext;
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
use siko_ir::types::DerivedClass;
use siko_ir::types::Record;
use siko_ir::types::TypeDef;
use siko_ir::types::TypeDefId;
use siko_ir::types::TypeSignature;
use siko_ir::walker::walk_expr;
use siko_ir::walker::Visitor;
use siko_location_info::item::LocationId;
#[allow(unused)]
use siko_util::format_list;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::dependency_processor::DependencyCollector;
use crate::dependency_processor::DependencyGroup;
use crate::dependency_processor::DependencyProcessor;

pub enum AutoDeriveState {
    No,
    Conditional(Vec<ClassId>),
    Yes,
}

pub struct Item {
    typedef_id: TypeDefId,
    class_id: ClassId,
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

    fn check_data_type(
        &mut self,
        type_arg_vars: Vec<TypeVariable>,
        data_type_var: TypeVariable,
        typedef_id: &TypeDefId,
        name: String,
        derived_class: ClassId,
        members: Vec<(TypeVariable, LocationId)>,
        errors: &mut Vec<TypecheckError>,
        derive_location: Option<LocationId>,
    ) {
        let resolution_result = self
            .type_store
            .has_class_instance(&data_type_var, &derived_class);
        match resolution_result {
            ResolutionResult::Definite(instance_id) => {
                if let Some(derive_location) = derive_location {
                    let instance = self.program.instances.get(&instance_id);
                    let class = self.program.classes.get(&instance.class_id);
                    let err = TypecheckError::AutoDeriveConflict(
                        name.clone(),
                        derive_location,
                        instance.location_id,
                        class.name.clone(),
                    );
                    errors.push(err);
                }
            }
            ResolutionResult::Inconclusive => unreachable!(),
            ResolutionResult::No => {}
        }
        let mut first_non_generic_instance = true;
        for member in &members {
            let ty = self.type_store.get_type(&member.0);
            match ty {
                Type::TypeArgument(_, constraints) => {
                    println!("Implementing instance with constraints: {:?}", constraints);
                }
                Type::FixedTypeArgument(_, _, constraints) => {
                    println!("Fixed!");
                }
                _ => {
                    let resolution_result = self
                        .type_store
                        .has_class_instance(&member.0, &derived_class);
                    //println!("{:?}", resolution_result);
                    for type_arg_var in &type_arg_vars {
                        let ty = self.type_store.get_type(type_arg_var);
                        match ty {
                            Type::TypeArgument(_, constraints) => {
                                println!(
                                    "Implementing instance with constraints: {:?}",
                                    constraints
                                );
                            }
                            Type::FixedTypeArgument(_, _, constraints) => {
                                println!("Fixed!");
                            }
                            _ => {
                                if first_non_generic_instance {
                                    let class = self.program.classes.get(&derived_class);
                                    println!("----->>>{:?} {} {}", ty, class.name, name);
                                    let err = TypecheckError::AutoDeriveMemberInstanceNotGeneric(
                                        name.clone(),
                                        member.1,
                                        class.name.clone(),
                                    );
                                    errors.push(err);
                                    first_non_generic_instance = false;
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut context = ResolverContext::new();
        let data_type = self
            .type_store
            .get_resolved_type_string_with_context(&data_type_var, &mut context);
        println!(
            "IMPLICIT ADT {}, has_class_instance {:?}",
            data_type, resolution_result
        );
    }

    fn check_adt(
        &mut self,
        adt: &Adt,
        derived_class: ClassId,
        errors: &mut Vec<TypecheckError>,
        derive_location: Option<LocationId>,
    ) {
        let adt_type_info = self
            .adt_type_info_map
            .get(&adt.id)
            .expect("Adt type info not found");
        let mut clone_context = self.type_store.create_clone_context();
        let mut type_arg_vars = Vec::new();
        for type_arg_var in &adt_type_info.type_arg_vars {
            let var = clone_context.clone_var(*type_arg_var);
            type_arg_vars.push(var);
        }
        let adt_type_var = clone_context.clone_var(adt_type_info.adt_type);
        let mut members = Vec::new();
        for index in 0..adt.variants.len() {
            let key = (adt.id, index);
            let variant_type_info = self
                .variant_type_info_map
                .get(&key)
                .expect("Variant type info not found");
            for item_type in &variant_type_info.item_types {
                let item_var = clone_context.clone_var(item_type.0);
                members.push((item_var, item_type.1));
            }
        }
        self.check_data_type(
            type_arg_vars,
            adt_type_var,
            &adt.id,
            adt.name.clone(),
            derived_class,
            members,
            errors,
            derive_location,
        );
    }

    fn check_record(
        &mut self,
        record: &Record,
        derived_class: ClassId,
        errors: &mut Vec<TypecheckError>,
        derive_location: Option<LocationId>,
    ) {
        let record_type_info = self
            .record_type_info_map
            .get(&record.id)
            .expect("Record type info not found");
        let mut clone_context = self.type_store.create_clone_context();
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
        self.check_data_type(
            type_arg_vars,
            record_type_var,
            &record.id,
            record.name.clone(),
            derived_class,
            members,
            errors,
            derive_location,
        );
    }

    pub fn process_functions(
        &mut self,
        errors: &mut Vec<TypecheckError>,
    ) -> Vec<DependencyGroup<TypeDefId>> {
        let mut typedefs = Vec::new();
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
                            self.check_adt(adt, *derived_class, errors, None);
                        }
                    }
                    AutoDeriveMode::Explicit(derived_classes) => {
                        for derived_class in derived_classes {
                            self.check_adt(
                                adt,
                                derived_class.class_id,
                                errors,
                                Some(derived_class.location_id),
                            );
                        }
                    }
                },
                TypeDef::Record(record) => match &record.auto_derive_mode {
                    AutoDeriveMode::Implicit => {
                        for derived_class in &implicit_derived_classes {
                            self.check_record(record, *derived_class, errors, None);
                        }
                    }
                    AutoDeriveMode::Explicit(derived_classes) => {
                        for derived_class in derived_classes {
                            self.check_record(
                                record,
                                derived_class.class_id,
                                errors,
                                Some(derived_class.location_id),
                            );
                        }
                    }
                },
            }
        }

        let dep_processor = DependencyProcessor::new(typedefs);
        let ordered_function_groups = dep_processor.process_items(self);

        ordered_function_groups
    }
}

impl<'a> DependencyCollector<TypeDefId> for TypedefDependencyProcessor<'a> {
    fn collect(&self, typedef_id: TypeDefId) -> Vec<TypeDefId> {
        let mut deps = BTreeSet::new();
        deps.into_iter().collect()
    }
}
