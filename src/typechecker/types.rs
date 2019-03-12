use super::function_type::FunctionType;
use super::type_variable::TypeVariable;
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
    Tuple(Vec<TypeVariable>),
    Function(FunctionType),
    TypeArgument { index: usize, user_defined: bool },
}

impl Type {
    pub fn clone_type_var(
        type_var: &TypeVariable,
        vars: &BTreeMap<TypeVariable, TypeVariable>,
        args: &BTreeMap<usize, usize>,
        type_store: &mut TypeStore,
    ) -> TypeVariable {
        let new_var = vars
            .get(type_var)
            .expect("Type variable not found during clone");
        let ty = type_store.get_type(type_var);
        let ty = ty.clone_type(vars, args, type_store);
        type_store.add_var_and_type(*new_var, ty);
        *new_var
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
            Type::Tuple(typevars) => {
                let typevars: Vec<_> = typevars
                    .iter()
                    .map(|var| Type::clone_type_var(var, vars, args, type_store))
                    .collect();
                Type::Tuple(typevars)
            }
            Type::Function(func_type) => {
                let types: Vec<_> = func_type
                    .type_vars
                    .iter()
                    .map(|var| Type::clone_type_var(var, vars, args, type_store))
                    .collect();
                Type::Function(FunctionType::new(types))
            }
            Type::TypeArgument {
                index,
                user_defined,
            } => {
                let new_index = args
                    .get(index)
                    .expect("Type argument not found during clone");
                Type::TypeArgument {
                    index: *new_index,
                    user_defined: *user_defined,
                }
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
            Type::Tuple(type_vars) => {
                for var in type_vars {
                    vars.push(*var);
                    let ty = type_store.get_type(var);
                    ty.collect(vars, args, type_store);
                }
            }
            Type::Function(func_type) => {
                for var in &func_type.type_vars {
                    vars.push(*var);
                    let ty = type_store.get_type(var);
                    ty.collect(vars, args, type_store);
                }
            }
            Type::TypeArgument { index: index, .. } => {
                args.push(*index);
            }
        }
    }

    pub fn as_string(&self, type_store: &TypeStore) -> String {
        match self {
            Type::Int => format!("Int"),
            Type::Float => format!("Float"),
            Type::Bool => format!("Bool"),
            Type::String => format!("String"),
            Type::Nothing => format!("!"),
            Type::Tuple(type_vars) => {
                let ss: Vec<_> = type_vars
                    .iter()
                    .map(|var| {
                        let ty = type_store.get_type(var);
                        ty.as_string(type_store)
                    })
                    .collect();
                format!("({})", ss.join(", "))
            }
            Type::Function(func_type) => func_type.as_string(type_store),
            Type::TypeArgument { index, .. } => format!("t{}", index),
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
                let ss: Vec<_> = types.iter().map(|t| format!("{:?}", t)).collect();
                write!(f, "({})", ss.join(", "))
            }
            Type::Function(func_type) => write!(f, "{}", func_type),
            Type::TypeArgument { index, .. } => write!(f, "t{}", index),
        }
    }
}
