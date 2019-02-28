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
    pub fn collect_type_arg(&self, used_type_args: &mut Vec<usize>) {
        match &self {
            Type::Int => {}
            Type::Bool => {}
            Type::String => {}
            Type::Nothing => {}
            Type::Tuple(types) => {
                for ty in types.iter() {
                    ty.collect_type_arg(used_type_args);
                }
            }
            Type::Function(func_type) => {
                func_type.collect_type_arg(used_type_args);
            }
            Type::TypeArgument(index) => {
                used_type_args.push(index.clone());
            }
            Type::TypeVar(var) => panic!("Collecing type arguments on unresolved type"),
        }
    }

    pub fn slice_to_string(types: &[Type]) -> String {
        let ss: Vec<_> = types.iter().map(|t| format!("{}", t)).collect();
        format!("({})", ss.join(", "))
    }

    pub fn remap_type_args(&mut self, arg_mapping: &BTreeMap<usize, usize>) {
        match self {
            Type::TypeArgument(i) => {
                *self = Type::TypeArgument(*arg_mapping.get(i).expect("Missing arg mapping"));
            }
            Type::Tuple(types) => {
                for ty in types {
                    ty.remap_type_args(arg_mapping);
                }
            }
            Type::Function(func_type) => {
                func_type.remap_type_args(arg_mapping);
            }
            _ => {}
        }
    }

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
