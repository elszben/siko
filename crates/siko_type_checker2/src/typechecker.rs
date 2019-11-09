use crate::error::Error;
use crate::error::TypecheckError;
use crate::instance_resolver::InstanceResolver;
use crate::substitution::Substitution;
use crate::type_var_generator::TypeVarGenerator;
use crate::types::BaseType;
use crate::types::Type;
use crate::unifier::Unifier;
use crate::util::create_general_function_type;
use siko_ir::class::ClassId;
use siko_ir::class::InstanceId;
use siko_ir::expr::ExprId;
use siko_ir::function::Function;
use siko_ir::function::FunctionInfo;
use siko_ir::function::NamedFunctionKind;
use siko_ir::program::Program;
use siko_ir::types::TypeDef;
use siko_ir::types::TypeDefId;
use siko_ir::types::TypeSignature;
use siko_ir::types::TypeSignatureId;
use siko_location_info::item::LocationId;
use siko_util::RcCounter;
use std::collections::BTreeMap;

pub struct DeriveInfo {
    class_id: ClassId,
    instance_index: usize,
}

pub struct AdtTypeInfo {
    adt_type: Type,
    variant_types: Vec<VariantTypeInfo>,
    derived_classes: Vec<DeriveInfo>,
}

pub struct VariantTypeInfo {
    item_types: Vec<(Type, LocationId)>,
}

pub struct RecordTypeInfo {
    record_type: Type,
    field_types: Vec<(Type, LocationId)>,
    derived_classes: Vec<DeriveInfo>,
}

pub struct FunctionTypeInfo {
    pub displayed_name: String,
    pub args: Vec<Type>,
    pub typed: bool,
    pub result: Type,
    pub function_type: Type,
    pub body: Option<ExprId>,
    pub location_id: LocationId,
}

impl FunctionTypeInfo {
    pub fn apply(&mut self, unifier: &Unifier) {
        for arg in &mut self.args {
            *arg = unifier.apply(arg);
        }
        self.result = unifier.apply(&self.result);
        self.function_type = unifier.apply(&self.function_type);
    }
}

fn process_type_signature(
    type_signature_id: TypeSignatureId,
    program: &Program,
    type_var_generator: &mut TypeVarGenerator,
) -> Type {
    let type_signature = &program.type_signatures.get(&type_signature_id).item;
    match type_signature {
        TypeSignature::Function(from, to) => {
            let from_ty = process_type_signature(*from, program, type_var_generator);
            let to_ty = process_type_signature(*to, program, type_var_generator);
            Type::Function(Box::new(from_ty), Box::new(to_ty))
        }
        TypeSignature::Named(name, id, items) => {
            let items: Vec<_> = items
                .iter()
                .map(|item| process_type_signature(*item, program, type_var_generator))
                .collect();
            Type::Named(name.clone(), *id, items)
        }
        TypeSignature::Tuple(items) => {
            let items: Vec<_> = items
                .iter()
                .map(|item| process_type_signature(*item, program, type_var_generator))
                .collect();
            Type::Tuple(items)
        }
        TypeSignature::TypeArgument(index, name, constraints) => {
            Type::FixedTypeArg(name.clone(), *index, constraints.clone())
        }
        TypeSignature::Variant(..) => panic!("Variant should not appear here"),
        TypeSignature::Wildcard => type_var_generator.get_new_type_var(),
    }
}

fn check_instance(
    class_id: ClassId,
    ty: &Type,
    location_id: LocationId,
    instance_resolver: &InstanceResolver,
    substitutions: &mut Vec<(Substitution, LocationId)>,
    mut type_var_generator: TypeVarGenerator,
) -> bool {
    //println!("Checking instance {} {}", class_id, ty);
    if let Type::Var(index, constraints) = ty {
        if constraints.contains(&class_id) {
            return true;
        } else {
            let mut new_constraints = constraints.clone();
            new_constraints.push(class_id);
            let new_type = Type::Var(type_var_generator.get_new_index(), new_constraints);
            let mut sub = Substitution::empty();
            sub.add(*index, &new_type).expect("sub add failed");
            substitutions.push((sub, location_id));
            return true;
        }
    }
    if let Some(sub) = instance_resolver.has_instance(&ty, class_id, type_var_generator.clone()) {
        let constraints = sub.get_constraints();
        substitutions.push((sub, location_id));
        for constraint in constraints {
            if constraint.ty.get_base_type() == BaseType::Generic {
                unimplemented!();
            } else {
                if !check_instance(
                    constraint.class_id,
                    &constraint.ty,
                    location_id,
                    instance_resolver,
                    substitutions,
                    type_var_generator.clone(),
                ) {
                    return false;
                }
            }
        }
        return true;
    } else {
        return false;
    }
}

fn process_type_change(
    target_ty: Type,
    source_index: usize,
    instance_resolver: &mut InstanceResolver,
    instance_index: usize,
    errors: &mut Vec<TypecheckError>,
    adt_name: &str,
    class_name: &str,
    location_id: LocationId,
) -> bool {
    let mut instance_changed = false;
    match target_ty {
        Type::Var(_, target_constraints) => {
            let mut instance = instance_resolver
                .get_auto_derived_instance(instance_index)
                .clone();
            let new_instance_ty = match instance.ty.clone() {
                Type::Named(name, id, args) => {
                    let mut new_args = Vec::new();
                    for arg in args {
                        match arg {
                            Type::Var(var_index, mut constraints) => {
                                if var_index == source_index {
                                    for t in &target_constraints {
                                        if constraints.contains(&t) {
                                            continue;
                                        } else {
                                            instance_changed = true;
                                            constraints.push(*t);
                                        }
                                    }
                                }
                                let new_type = Type::Var(var_index, constraints);
                                new_args.push(new_type);
                            }
                            _ => unreachable!(),
                        }
                    }
                    Type::Named(name, id, new_args)
                }
                _ => unreachable!(),
            };
            instance.ty = new_instance_ty;
            instance_resolver.update_auto_derived_instance(instance_index, instance);
        }
        _ => {
            let err = TypecheckError::DeriveFailureInstanceNotGeneric(
                adt_name.to_string(),
                class_name.to_string(),
                location_id,
            );
            errors.push(err);
        }
    }
    instance_changed
}

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
        type_var_generator: &mut TypeVarGenerator,
        adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
        record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    ) {
        instance_resolver.check_conflicts(errors, program, type_var_generator.clone());

        if !errors.is_empty() {
            return;
        }

        loop {
            let mut instance_changed = false;
            for (id, adt_type_info) in adt_type_info_map {
                let adt = program.typedefs.get(id).get_adt();
                for derive_info in &adt_type_info.derived_classes {
                    let class = program.classes.get(&derive_info.class_id);
                    //println!("Processing derived_class {} for {}", class.name, adt.name);
                    let mut substitutions = Vec::new();
                    for variant_type in &adt_type_info.variant_types {
                        for item_type in &variant_type.item_types {
                            if check_instance(
                                derive_info.class_id,
                                &item_type.0,
                                item_type.1,
                                &instance_resolver,
                                &mut substitutions,
                                type_var_generator.clone(),
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
                    for (sub, location_id) in substitutions {
                        for (index, target_ty) in sub.get_changes() {
                            if process_type_change(
                                target_ty.clone(),
                                *index,
                                instance_resolver,
                                derive_info.instance_index,
                                errors,
                                &adt.name,
                                &class.name,
                                location_id,
                            ) {
                                instance_changed = true;
                            }
                        }
                    }
                }
            }

            for (id, record_type_info) in record_type_info_map {
                let record = program.typedefs.get(id).get_record();
                for derive_info in &record_type_info.derived_classes {
                    let class = program.classes.get(&derive_info.class_id);
                    //println!("Processing derived_class {} for {}", class.name, record.name);
                    let mut substitutions = Vec::new();
                    for field_type in &record_type_info.field_types {
                        if check_instance(
                            derive_info.class_id,
                            &field_type.0,
                            field_type.1,
                            &instance_resolver,
                            &mut substitutions,
                            type_var_generator.clone(),
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
                    for (sub, location_id) in substitutions {
                        for (index, target_ty) in sub.get_changes() {
                            if process_type_change(
                                target_ty.clone(),
                                *index,
                                instance_resolver,
                                derive_info.instance_index,
                                errors,
                                &record.name,
                                &class.name,
                                location_id,
                            ) {
                                instance_changed = true;
                            }
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
        type_var_generator: &mut TypeVarGenerator,
        adt_type_info_map: &mut BTreeMap<TypeDefId, AdtTypeInfo>,
        record_type_info_map: &mut BTreeMap<TypeDefId, RecordTypeInfo>,
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
                                type_var_generator,
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
                    adt_type_info_map.insert(
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
                            type_var_generator,
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
                    record_type_info_map.insert(
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
            let ty = ty.remove_fixed_types();
            let ty = ty.add_constraints(&class.constraints);
            //println!("class type {}", ty);
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
        location_id: LocationId,
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
            location_id,
        };
        function_type_info
    }

    /*
    fn register_typed_function(
        &mut self,
        displayed_name: String,
        name: &String,
        arg_count: usize,
        type_signature_id: TypeSignatureId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
        body: Option<ExprId>,
        location_id: LocationId,
        kind: &NamedFunctionKind,
        class_member_type_info_map: &BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
    ) {
        let (arg_map, func_type_var) =
            if let NamedFunctionKind::DefaultClassMember(class_member_id) = kind {
                let info = class_member_type_info_map
                    .get(class_member_id)
                    .expect("class member type info not found");
                (info.arg_map.clone(), info.member_type_var)
            } else {
                let mut arg_map = BTreeMap::new();
                let func_type_var = process_type_signature(
                    &mut self.type_store,
                    &type_signature_id,
                    program,
                    &mut arg_map,
                    &mut None,
                );
                (arg_map, func_type_var)
            };
        let is_member = *kind != NamedFunctionKind::Free;

        /*
        println!(
            "Registering named function {} {} with type {}",
            function_id,
            displayed_name,
            self.type_store.get_resolved_type_string(&func_type_var, program)
        );
        */
        let ty = self.type_store.get_type(&func_type_var);
        let function_type_info = match ty {
            Type::Function(func_type) => {
                let mut signature_vars = Vec::new();
                func_type.get_arg_types(&self.type_store, &mut signature_vars);
                if signature_vars.len() < arg_count {
                    let location = if is_member {
                        location_id
                    } else {
                        program.type_signatures.get(&type_signature_id).location_id
                    };
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        name.clone(),
                        arg_count,
                        signature_vars.len(),
                        location,
                        is_member,
                    );
                    errors.push(err);
                    return;
                }
                let arg_vars: Vec<_> = signature_vars.iter().take(arg_count).cloned().collect();

                let return_value_var = func_type.get_return_type(&self.type_store, arg_vars.len());
                FunctionTypeInfo::new(
                    displayed_name,
                    arg_vars,
                    true,
                    return_value_var,
                    func_type_var,
                    body,
                    location_id,
                    arg_map,
                )
            }
            _ => {
                if arg_count > 0 {
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        displayed_name,
                        arg_count,
                        0,
                        program.type_signatures.get(&type_signature_id).location_id,
                        is_member,
                    );
                    errors.push(err);
                    return;
                }
                FunctionTypeInfo::new(
                    name.clone(),
                    vec![],
                    true,
                    func_type_var,
                    func_type_var,
                    body,
                    location_id,
                    arg_map,
                )
            }
        };
    }
    */

    fn process_functions(
        &self,
        program: &Program,
        type_var_generator: &mut TypeVarGenerator,
        adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
        record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
        errors: &mut Vec<TypecheckError>,
    ) {
        for (id, function) in &program.functions.items {
            let displayed_name = format!("{}", function.info);
            match &function.info {
                FunctionInfo::RecordConstructor(i) => {
                    let record = program.typedefs.get(&i.type_id).get_record();
                    let record_type_info = record_type_info_map
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
                        location_id: record.location_id,
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
                    println!("Record Final func_type {}", func_type_info.function_type);
                }
                FunctionInfo::VariantConstructor(i) => {
                    let adt = program.typedefs.get(&i.type_id).get_adt();
                    let adt_type_info = adt_type_info_map
                        .get(&i.type_id)
                        .expect("Adt type info not found");

                    let variant_type_info = &adt_type_info.variant_types[i.index];

                    let mut func_args = Vec::new();

                    let (func_type, result_type) = create_general_function_type(
                        &mut func_args,
                        variant_type_info.item_types.len(),
                        type_var_generator,
                    );

                    let location = program
                        .type_signatures
                        .get(&adt.variants[i.index].type_signature_id)
                        .location_id;

                    let mut func_type_info = FunctionTypeInfo {
                        displayed_name: format!("{}/{}_ctor", adt.name, adt.variants[i.index].name),
                        args: func_args.clone(),
                        typed: true,
                        result: result_type.clone(),
                        function_type: func_type,
                        body: None,
                        location_id: location,
                    };

                    for (index, item_type) in variant_type_info.item_types.iter().enumerate() {
                        let mut unifier = Unifier::new(type_var_generator.clone());
                        let arg_type = &func_args[index];
                        unifier.unify(&item_type.0, arg_type).expect("Unify failed");
                        func_type_info.apply(&unifier);
                    }

                    let mut unifier = Unifier::new(type_var_generator.clone());
                    unifier
                        .unify(&adt_type_info.adt_type, &result_type)
                        .expect("Unify failed");

                    func_type_info.apply(&unifier);
                    println!("Final func_type {}", func_type_info.function_type);
                }
                FunctionInfo::Lambda(i) => {
                    let func_type_info = self.register_untyped_function(
                        displayed_name,
                        function,
                        i.body,
                        i.location_id,
                        type_var_generator,
                    );
                }
                FunctionInfo::NamedFunction(i) => match i.type_signature {
                    Some(type_signature) => {
                        /*
                        self.register_typed_function(
                            displayed_name,
                            &i.name,
                            function.arg_locations.len(),
                            type_signature,
                            *id,
                            program,
                            errors,
                            i.body,
                            i.location_id,
                            &i.kind,
                            class_member_type_info_map,
                        );
                        */
                    }
                    None => match i.body {
                        Some(body) => {
                            let func_type_info = self.register_untyped_function(
                                displayed_name,
                                function,
                                body,
                                i.location_id,
                                type_var_generator,
                            );
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

    pub fn check(&self, program: &Program, counter: RcCounter) -> Result<(), Error> {
        let mut errors = Vec::new();
        let mut type_var_generator = TypeVarGenerator::new(counter);
        let mut class_types = BTreeMap::new();
        let mut instance_resolver = InstanceResolver::new();
        let mut adt_type_info_map = BTreeMap::new();
        let mut record_type_info_map = BTreeMap::new();

        self.process_classes_and_user_defined_instances(
            program,
            &mut type_var_generator,
            &mut instance_resolver,
            &mut class_types,
        );

        self.process_data_types(
            &mut instance_resolver,
            program,
            &mut type_var_generator,
            &mut adt_type_info_map,
            &mut record_type_info_map,
        );

        instance_resolver.check_conflicts(&mut errors, program, type_var_generator.clone());

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        self.process_derived_instances_instances(
            &mut instance_resolver,
            &mut errors,
            program,
            &mut type_var_generator,
            &adt_type_info_map,
            &record_type_info_map,
        );

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        self.process_functions(
            program,
            &mut type_var_generator,
            &adt_type_info_map,
            &record_type_info_map,
            &mut errors,
        );

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        Ok(())
    }
}
