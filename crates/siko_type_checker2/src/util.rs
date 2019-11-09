use crate::type_var_generator::TypeVarGenerator;
use crate::types::Type;

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
