use crate::function_type::FunctionType;
use crate::type_store::TypeStore;
use crate::type_variable::TypeVariable;
use crate::types::Type;
use siko_ir::class::InstanceId;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_location_info::item::LocationId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

pub struct ClassTypeVariableHandler {
    pub class_arg_index: usize,
    pub class_type_var: Option<TypeVariable>,
}

impl ClassTypeVariableHandler {
    pub fn new(index: usize) -> ClassTypeVariableHandler {
        ClassTypeVariableHandler {
            class_arg_index: index,
            class_type_var: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstanceTypeInfo {
    pub instance_id: InstanceId,
    pub type_var: TypeVariable,
    pub location_id: LocationId,
}

impl InstanceTypeInfo {
    pub fn new(
        instance_id: InstanceId,
        type_var: TypeVariable,
        location_id: LocationId,
    ) -> InstanceTypeInfo {
        InstanceTypeInfo {
            instance_id: instance_id,
            type_var: type_var,
            location_id: location_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClassMemberTypeInfo {
    pub member_type_var: TypeVariable,
    pub class_type_var: TypeVariable,
    pub arg_map: BTreeMap<usize, TypeVariable>,
}

#[derive(Debug, Clone)]
pub struct RecordTypeInfo {
    pub record_type: TypeVariable,
    pub field_types: Vec<TypeVariable>,
}

#[derive(Debug, Clone)]
pub struct VariantTypeInfo {
    pub variant_type: TypeVariable,
    pub item_types: Vec<TypeVariable>,
}

#[derive(Clone)]
pub struct FunctionTypeInfo {
    pub displayed_name: String,
    pub args: Vec<TypeVariable>,
    pub typed: bool,
    pub result: TypeVariable,
    pub function_type: TypeVariable,
    pub body: Option<ExprId>,
    pub location_id: LocationId,
    pub arg_map: BTreeMap<usize, TypeVariable>,
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
        arg_map: BTreeMap<usize, TypeVariable>,
    ) -> FunctionTypeInfo {
        FunctionTypeInfo {
            displayed_name: displayed_name,
            args: args,
            typed: typed,
            result: result,
            function_type: function_type,
            body: body,
            location_id: location_id,
            arg_map: arg_map,
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
