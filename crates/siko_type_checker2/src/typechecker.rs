use crate::error::TypecheckError;
use crate::types::Type;
use siko_ir::program::Program;
use siko_ir::types::TypeDef;
use siko_ir::types::TypeSignature;
use siko_ir::types::TypeSignatureId;
use siko_util::Counter;
use std::collections::BTreeMap;

pub struct TypeProcessor {
    counter: Counter,
}

impl TypeProcessor {
    pub fn new() -> TypeProcessor {
        TypeProcessor {
            counter: Counter::new(),
        }
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
            TypeSignature::Wildcard => Type::Var(self.counter.next(), Vec::new()),
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
        let mut all_instances = BTreeMap::new();
        for (class_id, class) in program.classes.items.iter() {
            // println!("Processing type for class {}", class.name);
            let type_signature_id = class.type_signature.expect("Class has no type signature");
            let ty = type_processor.process_type_signature(type_signature_id, program);
            let ty = ty.add_constraints(&class.constraints);
            //println!("class type {}", ty);
            class_types.insert(class_id, ty);
        }
        for (instance_id, instance) in program.instances.items.iter() {
            let ty = type_processor.process_type_signature(instance.type_signature, program);
            //println!("class instance ty {}", ty);
            let class_instances = all_instances
                .entry(instance.class_id)
                .or_insert_with(|| BTreeMap::new());
            let instances = class_instances
                .entry(ty.get_base_type())
                .or_insert_with(|| Vec::new());
            instances.push((ty, instance_id));
        }
        for (typedef_id, typedef) in program.typedefs.items.iter() {
            match typedef {
                TypeDef::Adt(adt) => {
                    let args: Vec<_> = adt
                        .type_args
                        .iter()
                        .map(|arg| Type::FixedTypeArg("<>".to_string(), *arg, Vec::new()))
                        .collect();
                    let ty = Type::Named(adt.name.clone(), *typedef_id, args);
                    println!("Processing {} => {}", adt.name, ty);
                    for variant in &adt.variants {
                        println!("  - {}", variant.name);
                        for item in &variant.items {
                            let ty = type_processor
                                .process_type_signature(item.type_signature_id, program);
                            println!("     {}", ty);
                        }
                    }
                }
                TypeDef::Record(record) => {}
            }
        }
        Ok(())
    }
}
