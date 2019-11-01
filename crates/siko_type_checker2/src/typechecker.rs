use crate::error::Error;
use crate::error::TypecheckError;
use crate::substitution::Constraint;
use crate::types::BaseType;
use crate::types::Type;
use crate::unifier::Unifier;
use siko_ir::class::ClassId;
use siko_ir::class::InstanceId;
use siko_ir::program::Program;
use siko_ir::types::DerivedClass;
use siko_ir::types::TypeDef;
use siko_ir::types::TypeSignature;
use siko_ir::types::TypeSignatureId;
use siko_location_info::item::LocationId;
use siko_util::Counter;
use std::collections::BTreeMap;

pub struct AdtTypeInfo {
    adt_type: Type,
    variant_types: Vec<VariantTypeInfo>,
    derived_classes: Vec<DerivedClass>,
}

pub struct VariantTypeInfo {
    item_types: Vec<(Type, LocationId)>,
}

pub enum InstanceInfo {
    UserDefined(Type, InstanceId, LocationId),
    AutoDerived(Type, LocationId),
}

impl InstanceInfo {
    pub fn get_type(&self) -> &Type {
        match self {
            InstanceInfo::UserDefined(ty, _, _) => &ty,
            InstanceInfo::AutoDerived(ty, _) => &ty,
        }
    }

    pub fn get_location(&self) -> LocationId {
        match self {
            InstanceInfo::UserDefined(_, _, id) => *id,
            InstanceInfo::AutoDerived(_, id) => *id,
        }
    }
}

pub struct InstanceResolver {
    instance_map: BTreeMap<ClassId, BTreeMap<BaseType, Vec<InstanceInfo>>>,
}

impl InstanceResolver {
    pub fn new() -> InstanceResolver {
        InstanceResolver {
            instance_map: BTreeMap::new(),
        }
    }

    pub fn add_user_defined(
        &mut self,
        class_id: ClassId,
        instance_ty: Type,
        instance_id: InstanceId,
        location_id: LocationId,
    ) {
        let class_instances = self
            .instance_map
            .entry(class_id)
            .or_insert_with(|| BTreeMap::new());
        let instances = class_instances
            .entry(instance_ty.get_base_type())
            .or_insert_with(|| Vec::new());
        instances.push(InstanceInfo::UserDefined(
            instance_ty,
            instance_id,
            location_id,
        ));
    }

    pub fn add_auto_derived(
        &mut self,
        class_id: ClassId,
        instance_ty: Type,
        location_id: LocationId,
    ) {
        let class_instances = self
            .instance_map
            .entry(class_id)
            .or_insert_with(|| BTreeMap::new());
        let instances = class_instances
            .entry(instance_ty.get_base_type())
            .or_insert_with(|| Vec::new());
        instances.push(InstanceInfo::AutoDerived(instance_ty, location_id));
    }

    pub fn has_instance(&self, ty: &Type, class_id: ClassId) -> Option<Vec<Constraint>> {
        let base_type = ty.get_base_type();
        if let Some(class_instances) = self.instance_map.get(&class_id) {
            if let Some(instances) = class_instances.get(&base_type) {
                for instance in instances {
                    let mut unifier = Unifier::new();
                    match instance {
                        InstanceInfo::AutoDerived(instance_ty, _) => {
                            if unifier.unify(ty, instance_ty).is_ok() {
                                return Some(unifier.get_constraints());
                            }
                        }
                        InstanceInfo::UserDefined(instance_ty, _, _) => {
                            if unifier.unify(ty, instance_ty).is_ok() {
                                return Some(unifier.get_constraints());
                            }
                        }
                    }
                }
                None
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn check_conflicts(&self, errors: &mut Vec<TypecheckError>, program: &Program) {
        for (class_id, class_instances) in &self.instance_map {
            let class = program.classes.get(&class_id);
            let mut first_generic_instance_location = None;
            if let Some(generic_instances) = class_instances.get(&BaseType::Generic) {
                first_generic_instance_location = Some(generic_instances[0].get_location());
            }
            for (_, instances) in class_instances {
                if let Some(generic_location) = first_generic_instance_location {
                    for instance in instances {
                        let other_instance_location = instance.get_location();
                        if other_instance_location == generic_location {
                            continue;
                        }
                        let err = TypecheckError::ConflictingInstances(
                            class.name.clone(),
                            generic_location,
                            other_instance_location,
                        );
                        errors.push(err);
                    }
                } else {
                    for (first_index, first_instance) in instances.iter().enumerate() {
                        for (second_index, second_instance) in instances.iter().enumerate() {
                            if first_index < second_index {
                                let first = first_instance.get_type();
                                let second = second_instance.get_type();
                                let mut unifier = Unifier::new();
                                if unifier.unify(first, second).is_ok() {
                                    let err = TypecheckError::ConflictingInstances(
                                        class.name.clone(),
                                        first_instance.get_location(),
                                        second_instance.get_location(),
                                    );
                                    errors.push(err);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct TypeProcessor {
    counter: Counter,
}

impl TypeProcessor {
    pub fn new() -> TypeProcessor {
        TypeProcessor {
            counter: Counter::new(),
        }
    }

    pub fn get_new_type_var(&mut self) -> Type {
        Type::Var(self.counter.next(), Vec::new())
    }

    pub fn process_type_signature(
        &mut self,
        type_signature_id: TypeSignatureId,
        program: &Program,
    ) -> Type {
        let type_signature = &program.type_signatures.get(&type_signature_id).item;
        match type_signature {
            TypeSignature::Function(from, to) => {
                let from_ty = self.process_type_signature(*from, program);
                let to_ty = self.process_type_signature(*to, program);
                Type::Function(Box::new(from_ty), Box::new(to_ty))
            }
            TypeSignature::Named(name, id, items) => {
                let items: Vec<_> = items
                    .iter()
                    .map(|item| self.process_type_signature(*item, program))
                    .collect();
                Type::Named(name.clone(), *id, items)
            }
            TypeSignature::Tuple(items) => {
                let items: Vec<_> = items
                    .iter()
                    .map(|item| self.process_type_signature(*item, program))
                    .collect();
                Type::Tuple(items)
            }
            TypeSignature::TypeArgument(index, name, constraints) => {
                Type::FixedTypeArg(name.clone(), *index, constraints.clone())
            }
            TypeSignature::Variant(..) => panic!("Variant should not appear here"),
            TypeSignature::Wildcard => self.get_new_type_var(),
        }
    }
}

fn check_instance(
    class_id: ClassId,
    ty: &Type,
    instance_resolver: &InstanceResolver,
    type_arg_constraints: &mut Vec<Constraint>,
) -> bool {
    println!("Checking instance {} {}", class_id, ty);
    if ty.get_base_type() == BaseType::Generic {
        type_arg_constraints.push(Constraint {
            class_id: class_id,
            ty: ty.clone(),
        });
        return true;
    }
    if let Some(constraints) = instance_resolver.has_instance(&ty, class_id) {
        for constraint in constraints {
            if constraint.ty.get_base_type() == BaseType::Generic {
                type_arg_constraints.push(constraint);
            } else {
                println!("Checking {:?}", constraint);
                if !check_instance(
                    constraint.class_id,
                    &constraint.ty,
                    instance_resolver,
                    type_arg_constraints,
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

pub struct Typechecker {}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {}
    }

    pub fn check(&self, program: &Program) -> Result<(), Error> {
        let mut errors = Vec::new();
        let mut type_processor = TypeProcessor::new();
        let mut class_types = BTreeMap::new();
        let mut instance_resolver = InstanceResolver::new();
        let mut adt_type_info_map = BTreeMap::new();
        for (class_id, class) in program.classes.items.iter() {
            // println!("Processing type for class {}", class.name);
            let type_signature_id = class.type_signature.expect("Class has no type signature");
            let ty = type_processor.process_type_signature(type_signature_id, program);
            let ty = ty.add_constraints(&class.constraints);
            //println!("class type {}", ty);
            class_types.insert(class_id, ty);
        }
        for (instance_id, instance) in program.instances.items.iter() {
            let instance_ty =
                type_processor.process_type_signature(instance.type_signature, program);

            instance_resolver.add_user_defined(
                instance.class_id,
                instance_ty,
                *instance_id,
                instance.location_id,
            );
        }
        for (typedef_id, typedef) in program.typedefs.items.iter() {
            match typedef {
                TypeDef::Adt(adt) => {
                    let args: Vec<_> = adt
                        .type_args
                        .iter()
                        .map(|arg| Type::FixedTypeArg("<>".to_string(), *arg, Vec::new()))
                        .collect();
                    let adt_type = Type::Named(adt.name.clone(), *typedef_id, args);
                    let mut variant_types = Vec::new();
                    for variant in adt.variants.iter() {
                        let mut item_types = Vec::new();
                        for item in variant.items.iter() {
                            let item_ty = type_processor
                                .process_type_signature(item.type_signature_id, program);
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
                    adt_type_info_map.insert(
                        adt.id,
                        AdtTypeInfo {
                            adt_type: adt_type.clone(),
                            variant_types: variant_types,
                            derived_classes: adt.derived_classes.clone(),
                        },
                    );
                    for derived_class in &adt.derived_classes {
                        let args: Vec<_> = adt
                            .type_args
                            .iter()
                            .map(|_| type_processor.get_new_type_var())
                            .collect();
                        let instance_ty = Type::Named(adt.name.clone(), *typedef_id, args);
                        instance_resolver.add_auto_derived(
                            derived_class.class_id,
                            instance_ty,
                            derived_class.location_id,
                        );
                    }
                }
                TypeDef::Record(record) => {}
            }
        }

        instance_resolver.check_conflicts(&mut errors, program);

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        for (id, adt_type_info) in &adt_type_info_map {
            let adt = program.typedefs.get(id).get_adt();
            for derived_class in &adt_type_info.derived_classes {
                let class = program.classes.get(&derived_class.class_id);
                println!("Processing derived_class {} for {}", class.name, adt.name);
                for variant_type in &adt_type_info.variant_types {
                    for item_type in &variant_type.item_types {
                        let mut type_arg_constraints = Vec::new();
                        if check_instance(
                            derived_class.class_id,
                            &item_type.0,
                            &instance_resolver,
                            &mut type_arg_constraints,
                        ) {
                        } else {
                            println!("{:?} does not implement {}", item_type.1, class.name);
                        }
                        for c in type_arg_constraints {
                            println!("type arg constraint {:?}", c);
                        }
                    }
                }
            }
        }

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        Ok(())
    }
}
