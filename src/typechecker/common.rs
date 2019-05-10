use crate::ir::expr::ExprId;
use crate::ir::function::FunctionId;
use crate::location_info::item::LocationId;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use std::collections::BTreeSet;
use std::fmt;

pub struct RecordFieldAccessorInfo {
    pub record_type: TypeVariable,
    pub field_types: Vec<TypeVariable>,
}

#[derive(Clone)]
pub struct DependencyGroup {
    pub functions: BTreeSet<FunctionId>,
}

impl DependencyGroup {
    pub fn new() -> DependencyGroup {
        DependencyGroup {
            functions: BTreeSet::new(),
        }
    }
}

pub struct FunctionTypeInfo {
    pub displayed_name: String,
    pub args: Vec<TypeVariable>,
    pub typed: bool,
    pub result: TypeVariable,
    pub function_type: TypeVariable,
    pub body: Option<ExprId>,
    pub location_id: LocationId,
}

impl FunctionTypeInfo {
    pub fn new(
        displayed_name: String,
        args: Vec<TypeVariable>,
        typed: bool,
        result: TypeVariable,
        function_type: TypeVariable,
        body: Option<ExprId>,
        location_id: LocationId,
    ) -> FunctionTypeInfo {
        FunctionTypeInfo {
            displayed_name: displayed_name,
            args: args,
            typed: typed,
            result: result,
            function_type: function_type,
            body: body,
            location_id: location_id,
        }
    }
}

impl fmt::Display for FunctionTypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut vars = self.args.clone();
        vars.push(self.result);
        let ss: Vec<_> = vars.iter().map(|i| format!("{}", i)).collect();
        write!(f, "{} = {}", self.function_type, ss.join(" -> "))
    }
}

pub fn create_general_function_type(
    arg_count: usize,
    args: &mut Vec<TypeVariable>,
    type_store: &mut TypeStore,
) -> (TypeVariable, TypeVariable) {
    if arg_count > 0 {
        let from_var = type_store.get_new_type_var();
        args.push(from_var);
        let (to_var, result) = create_general_function_type(arg_count - 1, args, type_store);
        let func_ty = Type::Function(FunctionType::new(from_var, to_var));
        let func_var = type_store.add_type(func_ty);
        (func_var, result)
    } else {
        let v = type_store.get_new_type_var();
        (v, v)
    }
}
