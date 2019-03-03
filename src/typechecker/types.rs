use super::function_type::FunctionType;
use super::type_variable::TypeVariable;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::type_store::TypeStore;
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

    pub fn clone_type(
        &self,
        vars: &BTreeMap<TypeVariable, TypeVariable>,
        args: &BTreeMap<usize, usize>,
        type_store: &mut TypeStore,
    ) -> Type {
        match self {
            Type::Int => self.clone(),
            Type::Bool => self.clone(),
            Type::String => self.clone(),
            Type::Nothing => self.clone(),
            Type::Tuple(types) => {}
            Type::Function(func_type) => {}
            Type::TypeArgument(index) => {}
            Type::TypeVar(var) => {}
        }
    }

    pub fn collect(&self, vars: &mut Vec<TypeVariable>, args: &mut Vec<usize>) {
        match self {
            Type::Int => {}
            Type::Bool => {}
            Type::String => {}
            Type::Nothing => {}
            Type::Tuple(types) => {
                types.iter().map(|t| t.collect(vars, args));
            }
            Type::Function(func_type) => {
                func_type.types.iter().map(|t| t.collect(vars, args));
            }
            Type::TypeArgument(index) => {
                args.push(*index);
            }
            Type::TypeVar(var) => {
                vars.push(*var);
            }
        }
    }

    pub fn as_string(&self, type_store: &TypeStore) -> String {
        if let Type::TypeVar(v) = self {
            let ty = type_store.get_resolved_type(v);
            format!("{}", ty)
        } else {
            match self {
                Type::Int => format!("Int"),
                Type::Bool => format!("Bool"),
                Type::String => format!("String"),
                Type::Nothing => format!("!"),
                Type::Tuple(types) => {
                    let ss: Vec<_> = types
                        .iter()
                        .map(|t| format!("{}", t.as_string(type_store)))
                        .collect();
                    format!("({})", ss.join(", "))
                }
                Type::Function(func_type) => func_type.as_string(type_store),
                Type::TypeArgument(index) => format!("t{}", index),
                Type::TypeVar(var) => format!("'{}", var.id),
            }
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
