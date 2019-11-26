use crate::common::AdtTypeInfo;
use crate::common::ClassMemberTypeInfo;
use crate::common::DeriveInfo;
use crate::common::FunctionTypeInfo;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::dependency_processor::DependencyGroup;
use crate::error::Error;
use crate::error::TypecheckError;
use crate::expression_checker::ExpressionChecker;
use crate::function_dep_processor::FunctionDependencyProcessor;
use crate::instance_resolver::InstanceResolver;
use crate::type_info_provider::TypeInfoProvider;
use crate::type_store::TypeStore;
use crate::type_store_initializer::TypeStoreInitializer;
use crate::undefined_var_checker::UndefinedVarChecker;
use crate::util::create_general_function_type;
use crate::util::process_type_signature;
use siko_ir::class::ClassId;
use siko_ir::data::TypeDef;
use siko_ir::expr::ExprId;
use siko_ir::function::Function;
use siko_ir::function::FunctionId;
use siko_ir::function::FunctionInfo;
use siko_ir::function::NamedFunctionKind;
use siko_ir::program::Program;
use siko_ir::type_var_generator::TypeVarGenerator;
use siko_ir::types::BaseType;
use siko_ir::types::Type;
use siko_ir::unifier::Unifier;
use siko_ir::walker::walk_expr;
use std::collections::BTreeMap;

pub struct Typechecker {}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {}
    }

    fn process_derived_instances_instances(
        &self,
        instance_resolver: &mut InstanceResolver,
        errors: &mut Vec<TypecheckError>,
        program: &Program,
        type_info_provider: &mut TypeInfoProvider,
    ) {
        instance_resolver.check_conflicts(
            errors,
            program,
            type_info_provider.type_var_generator.clone(),
        );

        if !errors.is_empty() {
            return;
        }

        loop {
            let mut instance_changed = false;
            for (id, adt_type_info) in &type_info_provider.adt_type_info_map {
                let adt = program.typedefs.get(id).get_adt();
                for derive_info in &adt_type_info.derived_classes {
                    let class = program.classes.get(&derive_info.class_id);
                    //println!("Processing derived_class {} for {}", class.name, adt.name);
                    let mut unifiers = Vec::new();
                    for variant_type in &adt_type_info.variant_types {
                        for item_type in &variant_type.item_types {
                            if instance_resolver.check_instance(
                                derive_info.class_id,
                                &item_type.0,
                                item_type.1,
                                &mut unifiers,
                                type_info_provider.type_var_generator.clone(),
                            ) {
                            } else {
                                let err = TypecheckError::DeriveFailureNoInstanceFound(
                                    adt.name.clone(),
                                    class.name.clone(),
                                    item_type.1,
                                );
                                errors.push(err);
                                //println!("{:?} does not implement {}", item_type.1, class.name);
                            }
                        }
                    }
                    for (unifier, location_id) in unifiers {
                        let mut instance = instance_resolver
                            .get_auto_derived_instance(derive_info.instance_index)
                            .clone();
                        if instance.ty.apply(&unifier) {
                            if let Type::Named(_, _, args) = &instance.ty {
                                for ty in args {
                                    if ty.get_base_type() != BaseType::Generic {
                                        let err = TypecheckError::DeriveFailureInstanceNotGeneric(
                                            adt.name.to_string(),
                                            class.name.to_string(),
                                            location_id,
                                        );
                                        errors.push(err);
                                        break;
                                    }
                                }
                            }
                            instance_changed = true;
                            instance_resolver
                                .update_auto_derived_instance(derive_info.instance_index, instance);
                        }
                    }
                }
            }

            for (id, record_type_info) in &type_info_provider.record_type_info_map {
                let record = program.typedefs.get(id).get_record();
                for derive_info in &record_type_info.derived_classes {
                    let class = program.classes.get(&derive_info.class_id);
                    //println!("Processing derived_class {} for {}", class.name, record.name);
                    let mut unifiers = Vec::new();
                    for field_type in &record_type_info.field_types {
                        if instance_resolver.check_instance(
                            derive_info.class_id,
                            &field_type.0,
                            field_type.1,
                            &mut unifiers,
                            type_info_provider.type_var_generator.clone(),
                        ) {
                        } else {
                            let err = TypecheckError::DeriveFailureNoInstanceFound(
                                record.name.clone(),
                                class.name.clone(),
                                field_type.1,
                            );
                            errors.push(err);
                            //println!("{:?} does not implement {}", item_type.1, class.name);
                        }
                    }
                    for (unifier, location_id) in unifiers {
                        let mut instance = instance_resolver
                            .get_auto_derived_instance(derive_info.instance_index)
                            .clone();
                        if instance.ty.apply(&unifier) {
                            if let Type::Named(_, _, args) = &instance.ty {
                                for ty in args {
                                    if ty.get_base_type() != BaseType::Generic {
                                        let err = TypecheckError::DeriveFailureInstanceNotGeneric(
                                            record.name.to_string(),
                                            class.name.to_string(),
                                            location_id,
                                        );
                                        errors.push(err);
                                        break;
                                    }
                                }
                            }
                            instance_changed = true;
                            instance_resolver
                                .update_auto_derived_instance(derive_info.instance_index, instance);
                        }
                    }
                }
            }

            if !instance_changed {
                break;
            }

            if !errors.is_empty() {
                break;
            }
        }
    }

    fn process_data_types(
        &self,
        instance_resolver: &mut InstanceResolver,
        program: &Program,
        type_info_provider: &mut TypeInfoProvider,
    ) {
        for (typedef_id, typedef) in program.typedefs.items.iter() {
            match typedef {
                TypeDef::Adt(adt) => {
                    let args: Vec<_> = adt
                        .type_args
                        .iter()
                        .map(|arg| Type::Var(*arg, Vec::new()))
                        .collect();
                    let adt_type = Type::Named(adt.name.clone(), *typedef_id, args.clone());
                    let mut variant_types = Vec::new();
                    for variant in adt.variants.iter() {
                        let mut item_types = Vec::new();
                        for item in variant.items.iter() {
                            let item_ty = process_type_signature(
                                item.type_signature_id,
                                program,
                                &mut type_info_provider.type_var_generator,
                            );
                            let item_ty = item_ty.remove_fixed_types();
                            let location = program
                                .type_signatures
                                .get(&item.type_signature_id)
                                .location_id;
                            item_types.push((item_ty, location));
                        }
                        variant_types.push(VariantTypeInfo {
                            item_types: item_types,
                        });
                    }
                    let mut derived_classes = Vec::new();
                    for derived_class in &adt.derived_classes {
                        let instance_ty = Type::Named(adt.name.clone(), *typedef_id, args.clone());
                        let instance_index = instance_resolver.add_auto_derived(
                            derived_class.class_id,
                            instance_ty,
                            derived_class.location_id,
                        );
                        let derive_info = DeriveInfo {
                            class_id: derived_class.class_id,
                            instance_index: instance_index,
                        };
                        derived_classes.push(derive_info);
                    }
                    type_info_provider.adt_type_info_map.insert(
                        adt.id,
                        AdtTypeInfo {
                            adt_type: adt_type,
                            variant_types: variant_types,
                            derived_classes: derived_classes,
                        },
                    );
                }
                TypeDef::Record(record) => {
                    let args: Vec<_> = record
                        .type_args
                        .iter()
                        .map(|arg| Type::Var(*arg, Vec::new()))
                        .collect();
                    let record_type = Type::Named(record.name.clone(), *typedef_id, args.clone());
                    let mut field_types = Vec::new();
                    for field in record.fields.iter() {
                        let field_ty = process_type_signature(
                            field.type_signature_id,
                            program,
                            &mut type_info_provider.type_var_generator,
                        );
                        let item_ty = field_ty.remove_fixed_types();
                        let location = program
                            .type_signatures
                            .get(&field.type_signature_id)
                            .location_id;
                        field_types.push((item_ty, location));
                    }
                    let mut derived_classes = Vec::new();
                    for derived_class in &record.derived_classes {
                        let instance_ty =
                            Type::Named(record.name.clone(), *typedef_id, args.clone());
                        let instance_index = instance_resolver.add_auto_derived(
                            derived_class.class_id,
                            instance_ty,
                            derived_class.location_id,
                        );
                        let derive_info = DeriveInfo {
                            class_id: derived_class.class_id,
                            instance_index: instance_index,
                        };
                        derived_classes.push(derive_info);
                    }
                    type_info_provider.record_type_info_map.insert(
                        record.id,
                        RecordTypeInfo {
                            record_type: record_type,
                            field_types: field_types,
                            derived_classes: derived_classes,
                        },
                    );
                }
            }
        }
    }

    fn process_classes_and_user_defined_instances(
        &self,
        program: &Program,
        type_var_generator: &mut TypeVarGenerator,
        instance_resolver: &mut InstanceResolver,
        class_types: &mut BTreeMap<ClassId, Type>,
    ) {
        for (class_id, class) in program.classes.items.iter() {
            // println!("Processing type for class {}", class.name);
            let type_signature_id = class.type_signature.expect("Class has no type signature");
            let ty = process_type_signature(type_signature_id, program, type_var_generator);
            let ty = ty.add_constraints(&class.constraints);
            // println!("class type {}", ty);
            class_types.insert(*class_id, ty);
        }
        for (instance_id, instance) in program.instances.items.iter() {
            let instance_ty =
                process_type_signature(instance.type_signature, program, type_var_generator);
            let instance_ty = instance_ty.remove_fixed_types();

            instance_resolver.add_user_defined(
                instance.class_id,
                instance_ty,
                *instance_id,
                instance.location_id,
            );
        }
    }

    fn register_untyped_function(
        &self,
        name: String,
        function: &Function,
        body: ExprId,
        type_var_generator: &mut TypeVarGenerator,
    ) -> FunctionTypeInfo {
        let mut args = Vec::new();

        let (func_type, result_type) = create_general_function_type(
            &mut args,
            function.arg_locations.len() + function.implicit_arg_count,
            type_var_generator,
        );
        let function_type_info = FunctionTypeInfo {
            displayed_name: name,
            args: args,
            typed: false,
            result: result_type,
            function_type: func_type,
            body: Some(body),
        };

        function_type_info
    }

    fn process_functions(
        &self,
        program: &Program,
        type_var_generator: &mut TypeVarGenerator,
        errors: &mut Vec<TypecheckError>,
        type_info_provider: &mut TypeInfoProvider,
    ) {
        for (id, function) in &program.functions.items {
            match &function.info {
                FunctionInfo::RecordConstructor(i) => {
                    let record = program.typedefs.get(&i.type_id).get_record();
                    let record_type_info = type_info_provider
                        .record_type_info_map
                        .get(&i.type_id)
                        .expect("record type info not found");
                    let mut func_args = Vec::new();

                    let (func_type, result_type) = create_general_function_type(
                        &mut func_args,
                        record.fields.len(),
                        type_var_generator,
                    );

                    let mut func_type_info = FunctionTypeInfo {
                        displayed_name: format!("{}_ctor", record.name),
                        args: func_args.clone(),
                        typed: true,
                        result: result_type.clone(),
                        function_type: func_type,
                        body: None,
                    };

                    for (index, field_type) in record_type_info.field_types.iter().enumerate() {
                        let mut unifier = Unifier::new(type_var_generator.clone());
                        let arg_type = &func_args[index];
                        unifier
                            .unify(&field_type.0, arg_type)
                            .expect("Unify failed");
                        func_type_info.apply(&unifier);
                    }

                    let mut unifier = Unifier::new(type_var_generator.clone());
                    unifier
                        .unify(&record_type_info.record_type, &result_type)
                        .expect("Unify failed");

                    func_type_info.apply(&unifier);
                    type_info_provider
                        .function_type_info_store
                        .add(*id, func_type_info);
                }
                FunctionInfo::VariantConstructor(i) => {
                    let adt = program.typedefs.get(&i.type_id).get_adt();
                    let mut adt_type_info = type_info_provider
                        .adt_type_info_map
                        .get(&i.type_id)
                        .expect("Adt type info not found")
                        .clone();

                    let mut variant_type_info = adt_type_info.variant_types[i.index].clone();

                    let mut func_args = Vec::new();

                    let (func_type, result_type) = create_general_function_type(
                        &mut func_args,
                        variant_type_info.item_types.len(),
                        type_var_generator,
                    );

                    let mut func_type_info = FunctionTypeInfo {
                        displayed_name: format!("{}/{}_ctor", adt.name, adt.variants[i.index].name),
                        args: func_args.clone(),
                        typed: true,
                        result: result_type.clone(),
                        function_type: func_type,
                        body: None,
                    };

                    let count = variant_type_info.item_types.len();
                    for index in 0..count {
                        let item_type = &variant_type_info.item_types[index];
                        let mut unifier = Unifier::new(type_var_generator.clone());
                        let arg_type = &func_type_info.args[index];
                        unifier.unify(&item_type.0, arg_type).expect("Unify failed");
                        func_type_info.apply(&unifier);
                        variant_type_info.apply(&unifier);
                        adt_type_info.apply(&unifier);
                    }

                    let mut unifier = Unifier::new(type_var_generator.clone());
                    unifier
                        .unify(&adt_type_info.adt_type, &func_type_info.result)
                        .expect("Unify failed");

                    func_type_info.apply(&unifier);
                    type_info_provider
                        .function_type_info_store
                        .add(*id, func_type_info);
                }
                FunctionInfo::Lambda(i) => {
                    let displayed_name = format!("{}", function.info);
                    let func_type_info = self.register_untyped_function(
                        displayed_name,
                        function,
                        i.body,
                        type_var_generator,
                    );
                    type_info_provider
                        .function_type_info_store
                        .add(*id, func_type_info);
                }
                FunctionInfo::NamedFunction(i) => match i.type_signature {
                    Some(type_signature) => {
                        let signature_ty =
                            process_type_signature(type_signature, program, type_var_generator);

                        let mut func_args = Vec::new();

                        let (func_type, result_type) = create_general_function_type(
                            &mut func_args,
                            function.arg_locations.len(),
                            type_var_generator,
                        );

                        let is_member = i.kind != NamedFunctionKind::Free;

                        let mut func_type_info = FunctionTypeInfo {
                            displayed_name: i.name.clone(),
                            args: func_args.clone(),
                            typed: true,
                            result: result_type.clone(),
                            function_type: func_type,
                            body: i.body,
                        };

                        let mut unifier = Unifier::new(type_var_generator.clone());
                        if unifier
                            .unify(&signature_ty, &func_type_info.function_type)
                            .is_err()
                        {
                            let err = TypecheckError::FunctionArgAndSignatureMismatch(
                                i.name.clone(),
                                func_args.len(),
                                signature_ty.get_arg_count(),
                                i.location_id,
                                is_member,
                            );
                            errors.push(err);
                        } else {
                            func_type_info.apply(&unifier);
                        }
                        type_info_provider
                            .function_type_info_store
                            .add(*id, func_type_info);
                    }
                    None => match i.body {
                        Some(body) => {
                            let displayed_name = format!("{}", function.info);
                            let func_type_info = self.register_untyped_function(
                                displayed_name,
                                function,
                                body,
                                type_var_generator,
                            );
                            type_info_provider
                                .function_type_info_store
                                .add(*id, func_type_info);
                        }
                        None => {
                            let err = TypecheckError::UntypedExternFunction(
                                i.name.clone(),
                                i.location_id,
                            );
                            errors.push(err)
                        }
                    },
                },
            }
        }
    }

    fn check_main(&self, program: &Program, errors: &mut Vec<TypecheckError>) {
        let mut main_found = false;

        for (_, function) in &program.functions.items {
            match &function.info {
                FunctionInfo::NamedFunction(info) => {
                    if info.module == siko_constants::MAIN_MODULE
                        && info.name == siko_constants::MAIN_FUNCTION
                    {
                        main_found = true;
                    }
                }
                _ => {}
            }
        }

        if !main_found {
            errors.push(TypecheckError::MainNotFound);
        }
    }

    fn init_expr_types<'a>(
        &self,
        function_id: &FunctionId,
        group: &'a DependencyGroup<FunctionId>,
        type_store: &'a mut TypeStore,
        type_info_provider: &'a mut TypeInfoProvider,
        program: &'a Program,
        errors: &'a mut Vec<TypecheckError>,
    ) {
        let function_type_info = type_info_provider
            .function_type_info_store
            .get_mut(function_id);
        let body = function_type_info.body.expect("body not found");
        let mut initializer =
            TypeStoreInitializer::new(program, group, type_store, type_info_provider, errors);
        walk_expr(&body, &mut initializer);
    }

    fn process_function<'a>(
        &self,
        function_id: &FunctionId,
        errors: &'a mut Vec<TypecheckError>,
        group: &'a DependencyGroup<FunctionId>,
        type_store: &'a mut TypeStore,
        type_info_provider: &'a mut TypeInfoProvider,
        instance_resolver: &'a mut InstanceResolver,
        program: &'a Program,
    ) {
        //let func = program.functions.get(function_id);
        //println!("Checking {}", func.info);
        let function_type_info = type_info_provider.function_type_info_store.get(function_id);
        let result_ty = function_type_info.result.clone();
        let body = function_type_info.body.expect("body not found");
        let mut checker = ExpressionChecker::new(
            program,
            group,
            type_store,
            type_info_provider,
            instance_resolver,
            errors,
        );
        walk_expr(&body, &mut checker);
        checker.match_expr_with(body, &result_ty);
        let function_type_info = type_info_provider.function_type_info_store.get(function_id);
        let mut func_args = Vec::new();
        function_type_info
            .function_type
            .collect_type_args(&mut func_args, program);
        let mut undef_var_checker =
            UndefinedVarChecker::new(program, type_store, errors, func_args);
        walk_expr(&body, &mut undef_var_checker);
    }

    fn process_dep_group<'a, 'b>(
        &self,
        group: &'b DependencyGroup<FunctionId>,
        errors: &'a mut Vec<TypecheckError>,
        type_store: &'a mut TypeStore,
        type_info_provider: &'a mut TypeInfoProvider,
        instance_resolver: &'a mut InstanceResolver,
        program: &'a Program,
    ) {
        for function in &group.items {
            self.init_expr_types(
                function,
                group,
                type_store,
                type_info_provider,
                program,
                errors,
            );
        }

        for function in &group.items {
            self.process_function(
                function,
                errors,
                group,
                type_store,
                type_info_provider,
                instance_resolver,
                program,
            );
        }
    }

    fn process_class_members(
        &self,
        program: &mut Program,
        type_info_provider: &mut TypeInfoProvider,
        class_types: &BTreeMap<ClassId, Type>,
    ) {
        for (class_member_id, class_member) in &program.class_members.items {
            let ty = process_type_signature(
                class_member.type_signature,
                program,
                &mut type_info_provider.type_var_generator,
            );
            let class_member_type_info = ClassMemberTypeInfo { ty: ty.clone() };
            program.class_member_types.insert(
                *class_member_id,
                (
                    ty,
                    class_types
                        .get(&class_member.class_id)
                        .expect("Class type not found")
                        .clone(),
                ),
            );
            type_info_provider
                .class_member_type_info_map
                .insert(*class_member_id, class_member_type_info);
        }
    }

    pub fn check(&self, program: &mut Program) -> Result<(), Error> {
        let mut errors = Vec::new();
        let mut type_var_generator = program.type_var_generator.clone();
        let mut type_info_provider = TypeInfoProvider::new(type_var_generator.clone());
        let mut class_types = BTreeMap::new();
        let mut instance_resolver =
            InstanceResolver::new(program.instance_resolution_cache.clone());

        self.process_classes_and_user_defined_instances(
            program,
            &mut type_var_generator,
            &mut instance_resolver,
            &mut class_types,
        );

        self.process_class_members(program, &mut type_info_provider, &class_types);

        self.process_data_types(&mut instance_resolver, program, &mut type_info_provider);

        instance_resolver.check_conflicts(&mut errors, program, type_var_generator.clone());

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        self.process_derived_instances_instances(
            &mut instance_resolver,
            &mut errors,
            program,
            &mut type_info_provider,
        );

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        self.process_functions(
            program,
            &mut type_var_generator,
            &mut errors,
            &mut type_info_provider,
        );

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        let function_dep_processor =
            FunctionDependencyProcessor::new(program, &type_info_provider.function_type_info_store);

        let ordered_dep_groups = function_dep_processor.process_functions();

        self.check_main(program, &mut errors);

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        for group in &ordered_dep_groups {
            let mut type_store = TypeStore::new();
            self.process_dep_group(
                group,
                &mut errors,
                &mut type_store,
                &mut type_info_provider,
                &mut instance_resolver,
                program,
            );
            //type_store.dump(program);
            type_store.save_expr_types(program);
        }

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        //type_info_provider.function_type_info_store.dump(program);

        type_info_provider
            .function_type_info_store
            .save_function_types(program);

        Ok(())
    }
}
