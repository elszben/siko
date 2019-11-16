use crate::type_var_generator::TypeVarGenerator;
use crate::types::Type;
use siko_ir::program::Program;
use siko_ir::types::TypeSignature;
use siko_ir::types::TypeSignatureId;

pub fn create_general_function_type(
    func_args: &mut Vec<Type>,
    arg_count: usize,
    type_var_generator: &mut TypeVarGenerator,
) -> (Type, Type) {
    if arg_count > 0 {
        let from_ty = type_var_generator.get_new_type_var();
        func_args.push(from_ty.clone());
        let (to_ty, result) =
            create_general_function_type(func_args, arg_count - 1, type_var_generator);
        let func_ty = Type::Function(Box::new(from_ty), Box::new(to_ty));
        (func_ty, result)
    } else {
        let v = type_var_generator.get_new_type_var();
        (v.clone(), v)
    }
}

pub fn get_list_type(program: &Program, ty: Type) -> Type {
    let id = program.get_named_type("Data.List", "List");
    Type::Named("List".to_string(), id, vec![ty])
}

pub fn get_string_type(program: &Program) -> Type {
    let id = program.get_named_type("Data.String", "String");
    Type::Named("String".to_string(), id, Vec::new())
}

pub fn get_bool_type(program: &Program) -> Type {
    let id = program.get_named_type("Data.Bool", "Bool");
    Type::Named("Bool".to_string(), id, Vec::new())
}

pub fn get_float_type(program: &Program) -> Type {
    let id = program.get_named_type("Data.Float", "Float");
    Type::Named("Float".to_string(), id, Vec::new())
}

pub fn get_int_type(program: &Program) -> Type {
    let id = program.get_named_type("Data.Int", "Int");
    Type::Named("Int".to_string(), id, Vec::new())
}

pub fn get_show_type(program: &Program, type_var_generator: &mut TypeVarGenerator) -> Type {
    let class_id = program
        .class_names
        .get("Show")
        .expect("Show not found")
        .clone();
    let index = type_var_generator.get_new_index();
    Type::Var(index, vec![class_id])
}

pub fn process_type_signature(
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
            let mut constraints = constraints.clone();
            // unifier assumes that the constraints are sorted!
            constraints.sort();
            Type::FixedTypeArg(name.clone(), *index, constraints)
        }
        TypeSignature::Variant(..) => panic!("Variant should not appear here"),
        TypeSignature::Wildcard => type_var_generator.get_new_type_var(),
    }
}
