use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FunctionType {
    pub from: TypeVariable,
    pub to: TypeVariable,
}

impl FunctionType {
    pub fn new(from: TypeVariable, to: TypeVariable) -> FunctionType {
        FunctionType { from: from, to: to }
    }

    pub fn get_return_type(&self, type_store: &TypeStore) -> TypeVariable {
        if let Type::Function(to_func_type) = type_store.get_type(&self.to) {
            to_func_type.get_return_type(type_store)
        } else {
            self.to
        }
    }

    pub fn get_arg_types(&self, type_store: &TypeStore, arg_vars: &mut Vec<TypeVariable>) {
        arg_vars.push(self.to);
        if let Type::Function(to_func_type) = type_store.get_type(&self.to) {
            to_func_type.get_arg_types(type_store, arg_vars);
        }
    }

    pub fn as_string(&self, type_store: &TypeStore) -> String {
        let from = type_store.get_type(&self.from).as_string(type_store, true);
        let to = type_store.get_type(&self.to).as_string(type_store, true);
        format!("{} -> {}", from, to)
    }
}

impl fmt::Display for FunctionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} -> {:?}", self.from, self.to)
    }
}
