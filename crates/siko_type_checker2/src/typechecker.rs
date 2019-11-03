use crate::error::Error;
use crate::error::TypecheckError;
use crate::substitution::Substitution;
use crate::types::BaseType;
use crate::types::Type;
use crate::unifier::Unifier;
use siko_ir::class::ClassId;
use siko_ir::class::InstanceId;
use siko_ir::program::Program;
use siko_ir::types::TypeDef;
use siko_ir::types::TypeSignature;
use siko_ir::types::TypeSignatureId;
use siko_location_info::item::LocationId;
use siko_util::RcCounter;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct AutDerivedInstance {
    ty: Type,
    location: LocationId,
}

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

pub enum InstanceInfo {
    UserDefined(Type, InstanceId, LocationId),
    AutoDerived(usize),
}

impl InstanceInfo {
    pub fn get_type<'a, 'b: 'a>(&'b self, instance_resolver: &'a InstanceResolver) -> &'a Type {
        match self {
            InstanceInfo::UserDefined(ty, _, _) => &ty,
            InstanceInfo::AutoDerived(index) => {
                &instance_resolver.get_auto_derived_instance(*index).ty
            }
        }
    }

    pub fn get_location(&self, instance_resolver: &InstanceResolver) -> LocationId {
        match self {
            InstanceInfo::UserDefined(_, _, id) => *id,
            InstanceInfo::AutoDerived(index) => {
                instance_resolver.get_auto_derived_instance(*index).location
            }
        }
    }
}

pub struct InstanceResolver {
    instance_map: BTreeMap<ClassId, BTreeMap<BaseType, Vec<InstanceInfo>>>,
    auto_derived_instances: Vec<AutDerivedInstance>,
}

impl InstanceResolver {
    pub fn new() -> InstanceResolver {
        InstanceResolver {
            instance_map: BTreeMap::new(),
            auto_derived_instances: Vec::new(),
        }
    }

    pub fn get_auto_derived_instance(&self, index: usize) -> &AutDerivedInstance {
        &self.auto_derived_instances[index]
    }

    pub fn update_auto_derived_instance(&mut self, index: usize, instance: AutDerivedInstance) {
        self.auto_derived_instances[index] = instance;
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
    ) -> usize {
        let class_instances = self
            .instance_map
            .entry(class_id)
            .or_insert_with(|| BTreeMap::new());
        let instances = class_instances
            .entry(instance_ty.get_base_type())
            .or_insert_with(|| Vec::new());
        let instance = AutDerivedInstance {
            ty: instance_ty,
            location: location_id,
        };
        let index = self.auto_derived_instances.len();
        self.auto_derived_instances.push(instance);
        instances.push(InstanceInfo::AutoDerived(index));
        index
    }

    pub fn has_instance(
        &self,
        ty: &Type,
        class_id: ClassId,
        type_var_generator: TypeVarGenerator,
    ) -> Option<Substitution> {
        let base_type = ty.get_base_type();
        if let Some(class_instances) = self.instance_map.get(&class_id) {
            if let Some(instances) = class_instances.get(&base_type) {
                for instance in instances {
                    let mut unifier = Unifier::new(type_var_generator.clone());
                    match instance {
                        InstanceInfo::AutoDerived(index) => {
                            let instance = self.get_auto_derived_instance(*index);
                            if unifier.unify(ty, &instance.ty).is_ok() {
                                return Some(unifier.get_substitution());
                            }
                        }
                        InstanceInfo::UserDefined(instance_ty, _, _) => {
                            if unifier.unify(ty, instance_ty).is_ok() {
                                return Some(unifier.get_substitution());
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

    pub fn check_conflicts(
        &self,
        errors: &mut Vec<TypecheckError>,
        program: &Program,
        type_var_generator: TypeVarGenerator,
    ) {
        for (class_id, class_instances) in &self.instance_map {
            let class = program.classes.get(&class_id);
            let mut first_generic_instance_location = None;
            if let Some(generic_instances) = class_instances.get(&BaseType::Generic) {
                first_generic_instance_location = Some(generic_instances[0].get_location(self));
            }
            for (_, instances) in class_instances {
                if let Some(generic_location) = first_generic_instance_location {
                    for instance in instances {
                        let other_instance_location = instance.get_location(self);
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
                                let first = first_instance.get_type(self);
                                let second = second_instance.get_type(self);
                                let mut unifier = Unifier::new(type_var_generator.clone());
                                if unifier.unify(first, second).is_ok() {
                                    let err = TypecheckError::ConflictingInstances(
                                        class.name.clone(),
                                        first_instance.get_location(self),
                                        second_instance.get_location(self),
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

#[derive(Clone)]
pub struct TypeVarGenerator {
    counter: RcCounter,
}

impl TypeVarGenerator {
    pub fn new(counter: RcCounter) -> TypeVarGenerator {
        TypeVarGenerator { counter: counter }
    }

    pub fn get_new_index(&mut self) -> usize {
        self.counter.next()
    }

    pub fn get_new_type_var(&mut self) -> Type {
        Type::Var(self.counter.next(), Vec::new())
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

    pub fn check(&self, program: &Program, counter: RcCounter) -> Result<(), Error> {
        let mut errors = Vec::new();
        let mut type_var_generator = TypeVarGenerator::new(counter);
        let mut class_types = BTreeMap::new();
        let mut instance_resolver = InstanceResolver::new();
        let mut adt_type_info_map = BTreeMap::new();
        for (class_id, class) in program.classes.items.iter() {
            // println!("Processing type for class {}", class.name);
            let type_signature_id = class.type_signature.expect("Class has no type signature");
            let ty = process_type_signature(type_signature_id, program, &mut type_var_generator);
            let ty = ty.remove_fixed_types();
            let ty = ty.add_constraints(&class.constraints);
            //println!("class type {}", ty);
            class_types.insert(class_id, ty);
        }
        for (instance_id, instance) in program.instances.items.iter() {
            let instance_ty =
                process_type_signature(instance.type_signature, program, &mut type_var_generator);
            let instance_ty = instance_ty.remove_fixed_types();

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
                                &mut type_var_generator,
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
                            adt_type: adt_type.clone(),
                            variant_types: variant_types,
                            derived_classes: derived_classes,
                        },
                    );
                }
                TypeDef::Record(record) => {}
            }
        }

        instance_resolver.check_conflicts(&mut errors, program, type_var_generator.clone());

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        loop {
            let mut instance_changed = false;
            for (id, adt_type_info) in &adt_type_info_map {
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
                            process_type_change(
                                target_ty.clone(),
                                *index,
                                &mut instance_resolver,
                                derive_info.instance_index,
                                &mut errors,
                                &adt.name,
                                &class.name,
                                location_id,
                            );
                        }
                    }
                }
            }

            if !instance_changed {
                break;
            }

            if !errors.is_empty() {
                return Err(Error::typecheck_err(errors));
            }
        }

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        Ok(())
    }
}
