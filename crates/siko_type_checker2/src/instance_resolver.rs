use crate::error::TypecheckError;
use crate::type_var_generator::TypeVarGenerator;
use crate::types::BaseType;
use crate::types::Type;
use crate::unifier::Unifier;
use siko_ir::class::ClassId;
use siko_ir::class::InstanceId;
use siko_ir::program::Program;
use siko_location_info::item::LocationId;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct AutoDerivedInstance {
    pub ty: Type,
    pub location: LocationId,
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
    auto_derived_instances: Vec<AutoDerivedInstance>,
}

impl InstanceResolver {
    pub fn new() -> InstanceResolver {
        InstanceResolver {
            instance_map: BTreeMap::new(),
            auto_derived_instances: Vec::new(),
        }
    }

    pub fn get_auto_derived_instance(&self, index: usize) -> &AutoDerivedInstance {
        &self.auto_derived_instances[index]
    }

    pub fn update_auto_derived_instance(&mut self, index: usize, instance: AutoDerivedInstance) {
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
        let instance = AutoDerivedInstance {
            ty: instance_ty,
            location: location_id,
        };
        let index = self.auto_derived_instances.len();
        self.auto_derived_instances.push(instance);
        instances.push(InstanceInfo::AutoDerived(index));
        index
    }

    fn has_instance(
        &self,
        ty: &Type,
        class_id: ClassId,
        type_var_generator: TypeVarGenerator,
    ) -> Option<Unifier> {
        let base_type = ty.get_base_type();
        if let Some(class_instances) = self.instance_map.get(&class_id) {
            if let Some(instances) = class_instances.get(&base_type) {
                for instance in instances {
                    let mut unifier = Unifier::new(type_var_generator.clone());
                    match instance {
                        InstanceInfo::AutoDerived(index) => {
                            let instance = self.get_auto_derived_instance(*index);
                            if unifier.unify(ty, &instance.ty).is_ok() {
                                return Some(unifier);
                            }
                        }
                        InstanceInfo::UserDefined(instance_ty, _, _) => {
                            if unifier.unify(ty, instance_ty).is_ok() {
                                return Some(unifier);
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

    pub fn check_instance(
        &self,
        class_id: ClassId,
        ty: &Type,
        location_id: LocationId,
        unifiers: &mut Vec<(Unifier, LocationId)>,
        mut type_var_generator: TypeVarGenerator,
    ) -> bool {
        //println!("Checking instance {} {}", class_id, ty);
        if let Type::Var(_, constraints) = ty {
            if constraints.contains(&class_id) {
                return true;
            } else {
                let mut new_constraints = constraints.clone();
                new_constraints.push(class_id);
                let new_type = Type::Var(type_var_generator.get_new_index(), new_constraints);
                let mut unifier = Unifier::new(type_var_generator.clone());
                let r = unifier.unify(&new_type, ty);
                assert!(r.is_ok());
                unifiers.push((unifier, location_id));
                return true;
            }
        }
        if let Some(unifier) = self.has_instance(&ty, class_id, type_var_generator.clone()) {
            let constraints = unifier.get_substitution().get_constraints();
            for constraint in constraints {
                if constraint.ty.get_base_type() == BaseType::Generic {
                    unimplemented!();
                } else {
                    if !self.check_instance(
                        constraint.class_id,
                        &constraint.ty,
                        location_id,
                        unifiers,
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
}
