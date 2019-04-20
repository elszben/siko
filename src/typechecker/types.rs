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
    FixedTypeArgument(usize, String),
    TupleFieldIndexable,
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
                let from = Type::clone_type_var(&func_type.from, vars, args, type_store);
                let to = Type::clone_type_var(&func_type.to, vars, args, type_store);
                Type::Function(FunctionType::new(from, to))
            }
            Type::TypeArgument(index) => {
                let new_index = args
                    .get(index)
                    .expect("Type argument not found during clone");
                Type::TypeArgument(*new_index)
            }
            Type::FixedTypeArgument(index, _) => {
                let new_index = args
                    .get(index)
                    .expect("Type argument not found during clone");
                Type::TypeArgument(*new_index)
            }
            Type::TupleFieldIndexable => Type::TupleFieldIndexable,
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
                for var in &[func_type.from, func_type.to] {
                    vars.push(*var);
                    let ty = type_store.get_type(var);
                    ty.collect(vars, args, type_store);
                }
            }
            Type::TypeArgument(index) => {
                args.push(*index);
            }
            Type::FixedTypeArgument(index, _) => args.push(*index),
            Type::TupleFieldIndexable => {}
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
            Type::FixedTypeArgument(_, name) => format!("{}", name),
            Type::TupleFieldIndexable => format!("<tuple>"),
        }
    }

    pub fn check_recursion(&self, vars: &Vec<TypeVariable>, type_store: &TypeStore) -> bool {
        fn check_sub_var(
            var: &TypeVariable,
            vars: &Vec<TypeVariable>,
            type_store: &TypeStore,
        ) -> bool {
            if vars.contains(var) {
                return true;
            }
            if type_store.is_recursive(*var) {
                return true;
            }
            false
        }

        match self {
            Type::Int => false,
            Type::Float => false,
            Type::Bool => false,
            Type::String => false,
            Type::Nothing => false,
            Type::Tuple(type_vars) => {
                for var in type_vars {
                    if check_sub_var(var, vars, type_store) {
                        return true;
                    }
                }
                false
            }
            Type::Function(func_type) => {
                if check_sub_var(&func_type.from, vars, type_store) {
                    return true;
                }
                check_sub_var(&func_type.to, vars, type_store)
            }
            Type::TypeArgument(..) => false,
            Type::FixedTypeArgument(..) => false,
            Type::TupleFieldIndexable => false,
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
            Type::FixedTypeArgument(_, name) => write!(f, "{}", name),
            Type::TupleFieldIndexable => write!(f, "<tuple>"),
        }
    }
}
