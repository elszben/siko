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
    TypeArgument(usize),
}

impl Type {
    pub fn clone_type_var(
        type_var: &TypeVariable,
        bindings: &mut BTreeMap<usize, TypeVariable>,
        type_store: &mut TypeStore,
    ) -> TypeVariable {
        let ty = type_store.get_type(type_var);
        let ty = ty.clone_type(bindings, type_store);
        type_store.add_var(ty)
    }

    pub fn clone_type(
        &self,
        bindings: &mut BTreeMap<usize, TypeVariable>,
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
                    .map(|var| Type::clone_type_var(var, bindings, type_store))
                    .collect();
                Type::Tuple(typevars)
            }
            Type::Function(func_type) => {
                let from = Type::clone_type_var(&func_type.from, bindings, type_store);
                let to = Type::clone_type_var(&func_type.to, bindings, type_store);
                Type::Function(FunctionType::new(from, to))
            }
            Type::TypeArgument(index) => {
                let var = bindings
                    .entry(*index)
                    .or_insert_with(|| type_store.get_new_type_var());
                type_store.get_type(var)
            }
        }
    }

    pub fn as_string(&self, type_store: &TypeStore, need_parens: bool) -> String {
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
                        ty.as_string(type_store, false)
                    })
                    .collect();
                format!("({})", ss.join(", "))
            }
            Type::Function(func_type) => {
                let func_type_str = func_type.as_string(type_store);
                if need_parens {
                    format!("({})", func_type_str)
                } else {
                    func_type_str
                }
            }
            Type::TypeArgument(index) => format!("t{}", index),
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
            Type::TypeArgument(index) => write!(f, "t{}", index),
        }
    }
}
