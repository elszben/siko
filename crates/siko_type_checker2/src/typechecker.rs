use crate::error::TypecheckError;
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
    UserDefined(Type, InstanceId),
    AutoDerived(Type, LocationId),
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
    ) {
        let class_instances = self
            .instance_map
            .entry(class_id)
            .or_insert_with(|| BTreeMap::new());
        let instances = class_instances
            .entry(instance_ty.get_base_type())
            .or_insert_with(|| Vec::new());
        instances.push(InstanceInfo::UserDefined(instance_ty, instance_id));
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

    pub fn has_instance(&self, ty: &Type, class_id: ClassId) -> bool {
        let base_type = ty.get_base_type();
        if let Some(class_instances) = self.instance_map.get(&class_id) {
            if let Some(instances) = class_instances.get(&base_type) {
                for instance in instances {
                    let mut unifier = Unifier::new();
                    match instance {
                        InstanceInfo::AutoDerived(instance_ty, _) => {
                            if unifier.unify(ty, instance_ty).is_ok() {
                                return true;
                            }
                        }
                        InstanceInfo::UserDefined(instance_ty, _) => {
                            if unifier.unify(ty, instance_ty).is_ok() {
                                unifier.dump();
                                return true;
                            }
                        }
                    }
                }
                false
            } else {
                false
            }
        } else {
            false
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

pub struct Typechecker {}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {}
    }

    pub fn check(&self, program: &Program) -> Result<(), TypecheckError> {
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
            instance_resolver.add_user_defined(instance.class_id, instance_ty, *instance_id);
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

        for (id, adt_type_info) in &adt_type_info_map {
            let adt = program.typedefs.get(id).get_adt();
            for derived_class in &adt_type_info.derived_classes {
                let class = program.classes.get(&derived_class.class_id);
                println!("Processing derived_class {} for {}", class.name, adt.name);
                for variant_type in &adt_type_info.variant_types {
                    for item_type in &variant_type.item_types {
                        if !instance_resolver.has_instance(&item_type.0, derived_class.class_id) {
                            println!("{:?} does not implement {}", item_type.1, class.name);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
