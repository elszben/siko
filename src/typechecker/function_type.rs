use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FunctionType {
    pub type_vars: Vec<TypeVariable>,
}

impl FunctionType {
    pub fn new(type_vars: Vec<TypeVariable>) -> FunctionType {
        FunctionType {
            type_vars: type_vars,
        }
    }

    pub fn get_return_type(&self) -> TypeVariable {
        self.type_vars.last().expect("empty function!").clone()
    }

    pub fn get_arg_count(&self) -> usize {
        self.type_vars.len() - 1
    }

    pub fn get_arg_types(&self) -> Vec<TypeVariable> {
        self.type_vars[0..self.type_vars.len() - 1].to_vec()
    }

    pub fn as_string(&self, type_store: &TypeStore) -> String {
        if self.get_arg_count() != 0 {
            let ss: Vec<_> = self
                .type_vars
                .iter()
                .map(|var| {
                    let ty = type_store.get_type(var);
                    ty.as_string(type_store)
                })
                .collect();
            format!("{}", ss.join(" -> "))
        } else {
            format!("{:?}", self.get_return_type())
        }
    }
}

impl fmt::Display for FunctionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.get_arg_count() != 0 {
            let ss: Vec<_> = self.type_vars.iter().map(|t| format!("{:?}", t)).collect();
            write!(f, "{}", ss.join(" -> "))
        } else {
            write!(f, "{:?}", self.get_return_type())
        }
    }
}
