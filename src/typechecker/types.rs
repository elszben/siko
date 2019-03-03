use super::function_type::FunctionType;
use super::type_variable::TypeVariable;
use crate::typechecker::error::TypecheckError;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Type {
    Int,
    Bool,
    String,
    Nothing,
    Tuple(Vec<Type>),
    Function(FunctionType),
    TypeArgument(usize),
    TypeVar(TypeVariable),
}

impl Type {
    pub fn get_inner_type_var(&self) -> TypeVariable {
        if let Type::TypeVar(v) = self {
            *v
        } else {
            unreachable!()
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
            Type::Nothing => write!(f, "!"),
            Type::Tuple(types) => {
                let ss: Vec<_> = types.iter().map(|t| format!("{}", t)).collect();
                write!(f, "({})", ss.join(", "))
            }
            Type::Function(func_type) => write!(f, "{}", func_type),
            Type::TypeArgument(index) => write!(f, "t{}", index),
            Type::TypeVar(var) => write!(f, "'{}", var.id),
        }
    }
}
