use crate::typechecker::type_store::TypeStore;
use crate::typechecker::types::Type;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FunctionType {
    pub types: Vec<Type>,
}

impl FunctionType {
    pub fn new(types: Vec<Type>) -> FunctionType {
        FunctionType { types: types }
    }

    pub fn get_return_type(&self) -> Type {
        self.types.last().expect("empty function!").clone()
    }

    pub fn get_arg_count(&self) -> usize {
        self.types.len() - 1
    }

    pub fn get_arg_types(&self) -> Vec<Type> {
        self.types[0..self.types.len() - 1].to_vec()
    }

    pub fn as_string(&self, type_store: &TypeStore) -> String {
        if self.get_arg_count() != 0 {
            let ss: Vec<_> = self
                .types
                .iter()
                .map(|t| format!("{}", t.as_string(type_store)))
                .collect();
            format!("{}", ss.join(" -> "))
        } else {
            format!("{}", self.get_return_type().as_string(type_store))
        }
    }
}

impl fmt::Display for FunctionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.get_arg_count() != 0 {
            let ss: Vec<_> = self.types.iter().map(|t| format!("{}", t)).collect();
            write!(f, "{}", ss.join(" -> "))
        } else {
            write!(f, "{}", self.get_return_type())
        }
    }
}
