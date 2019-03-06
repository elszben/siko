use super::function_type::FunctionType;
use super::type_variable::TypeVariable;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::type_store::TypeStore;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Type {
    Int,
    Float,
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
            Type::Float => self.clone(),
            Type::Bool => self.clone(),
            Type::String => self.clone(),
            Type::Nothing => self.clone(),
            Type::Tuple(types) => {
                let types: Vec<_> = types
                    .iter()
                    .map(|ty| ty.clone_type(vars, args, type_store))
                    .collect();
                Type::Tuple(types)
            }
            Type::Function(func_type) => {
                let types: Vec<_> = func_type
                    .types
                    .iter()
                    .map(|ty| ty.clone_type(vars, args, type_store))
                    .collect();
                Type::Function(FunctionType::new(types))
            }
            Type::TypeArgument(index) => {
                let new_index = args
                    .get(index)
                    .expect("Type argument not found during clone");
                Type::TypeArgument(*new_index)
            }
            Type::TypeVar(var) => {
                let new_var = vars.get(var).expect("Type variable not found during clone");
                let ty = type_store.get_type(var);
                let ty = ty.clone_type(vars, args, type_store);
                type_store.add_var_and_type(*new_var, ty);
                Type::TypeVar(*new_var)
            }
        }
    }

    pub fn collect(
        &self,
        vars: &mut Vec<TypeVariable>,
        args: &mut Vec<usize>,
        type_store: &TypeStore,
    ) {
        match self {
            Type::Int => {}
            Type::Float => {}
            Type::Bool => {}
            Type::String => {}
            Type::Nothing => {}
            Type::Tuple(types) => {
                for ty in types {
                    ty.collect(vars, args, type_store);
                }
            }
            Type::Function(func_type) => {
                for ty in &func_type.types {
                    ty.collect(vars, args, type_store);
                }
            }
            Type::TypeArgument(index) => {
                args.push(*index);
            }
            Type::TypeVar(var) => {
                let ty = type_store.get_type(var);
                vars.push(*var);
                ty.collect(vars, args, type_store);
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
                Type::Float => format!("Float"),
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
            Type::Float => write!(f, "Float"),
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
