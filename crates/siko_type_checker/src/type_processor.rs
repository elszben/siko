use crate::function_type::FunctionType;
use crate::type_store::TypeStore;
use crate::type_variable::TypeVariable;
use crate::types::Type;
use siko_ir::program::Program;
use siko_ir::types::TypeSignature;
use siko_ir::types::TypeSignatureId;

use std::collections::BTreeMap;

pub fn process_type_signature(
    type_store: &mut TypeStore,
    type_signature_id: &TypeSignatureId,
    program: &Program,
    arg_map: &mut BTreeMap<usize, TypeVariable>,
) -> TypeVariable {
    let type_signature = &program.type_signatures.get(type_signature_id).item;
    match type_signature {
        TypeSignature::Bool => {
            let ty = Type::Bool;
            return type_store.add_type(ty);
        }
        TypeSignature::Int => {
            let ty = Type::Int;
            return type_store.add_type(ty);
        }
        TypeSignature::String => {
            let ty = Type::String;
            return type_store.add_type(ty);
        }
        TypeSignature::Nothing => {
            let ty = Type::Nothing;
            return type_store.add_type(ty);
        }
        TypeSignature::Tuple(items) => {
            let items: Vec<_> = items
                .iter()
                .map(|i| process_type_signature(type_store, i, program, arg_map))
                .collect();
            let ty = Type::Tuple(items);
            return type_store.add_type(ty);
        }
        TypeSignature::Function(from, to) => {
            let from_var = process_type_signature(type_store, from, program, arg_map);
            let to_var = process_type_signature(type_store, to, program, arg_map);
            let ty = Type::Function(FunctionType::new(from_var, to_var));
            return type_store.add_type(ty);
        }
        TypeSignature::TypeArgument(index, name, constraints) => {
            let var = arg_map.entry(*index).or_insert_with(|| {
                let arg = type_store.get_unique_type_arg();
                let ty = Type::FixedTypeArgument(arg, name.clone(), constraints.clone());
                type_store.add_type(ty)
            });
            return *var;
        }
        TypeSignature::Named(name, id, items) => {
            let items: Vec<_> = items
                .iter()
                .map(|i| process_type_signature(type_store, i, program, arg_map))
                .collect();
            let ty = Type::Named(name.clone(), *id, items);
            return type_store.add_type(ty);
        }
        TypeSignature::Variant(..) => unreachable!(),
        TypeSignature::Wildcard => {
            let arg = type_store.get_unique_type_arg();
            let ty = Type::TypeArgument(arg, vec![]);
            type_store.add_type(ty)
        }
    }
}
