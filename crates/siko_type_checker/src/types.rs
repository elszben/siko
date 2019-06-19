use super::function_type::FunctionType;
use super::type_variable::TypeVariable;
use crate::type_store::CloneContext;
use crate::type_store::TypeIndex;
use crate::type_store::TypeStore;
use siko_ir::class::ClassId;
use siko_ir::types::TypeDefId;
use siko_util::Collector;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Type {
    Nothing,
    Tuple(Vec<TypeVariable>),
    Function(FunctionType),
    TypeArgument(usize, Vec<ClassId>),
    FixedTypeArgument(usize, String, Vec<ClassId>),
    Named(String, TypeDefId, Vec<TypeVariable>),
}

impl Type {
    pub fn clone_type_var(type_var: &TypeVariable, context: &mut CloneContext) -> TypeVariable {
        let new_var = context.var(*type_var);
        let old_index = context.type_store.get_index(type_var);
        let new_index = context.index(old_index);
        let old_type = context.type_store.get_type(type_var);
        let new_ty = old_type.clone_type(context);
        context
            .type_store
            .add_var_and_type(new_var, new_ty, new_index);
        new_var
    }

    pub fn clone_type(&self, context: &mut CloneContext) -> Type {
        match self {
            Type::Nothing => self.clone(),
            Type::Tuple(typevars) => {
                let typevars: Vec<_> = typevars
                    .iter()
                    .map(|var| Type::clone_type_var(var, context))
                    .collect();
                Type::Tuple(typevars)
            }
            Type::Function(func_type) => {
                let from = Type::clone_type_var(&func_type.from, context);
                let to = Type::clone_type_var(&func_type.to, context);
                Type::Function(FunctionType::new(from, to))
            }
            Type::TypeArgument(index, constraints) => {
                let new_index = context.arg(*index);
                Type::TypeArgument(new_index, constraints.clone())
            }
            Type::FixedTypeArgument(index, _, constraints) => {
                let new_index = context.arg(*index);
                Type::TypeArgument(new_index, constraints.clone())
            }
            Type::Named(name, id, typevars) => {
                let typevars: Vec<_> = typevars
                    .iter()
                    .map(|var| Type::clone_type_var(var, context))
                    .collect();
                Type::Named(name.clone(), id.clone(), typevars)
            }
        }
    }

    pub fn collect(
        &self,
        vars: &mut BTreeSet<TypeVariable>,
        args: &mut BTreeSet<usize>,
        indices: &mut BTreeSet<TypeIndex>,
        constraints: &mut Collector<usize, ClassId>,
        type_store: &TypeStore,
    ) {
        match self {
            Type::Nothing => {}
            Type::Tuple(type_vars) => {
                for var in type_vars {
                    vars.insert(*var);
                    indices.insert(type_store.get_index(var));
                    let ty = type_store.get_type(var);
                    ty.collect(vars, args, indices, constraints, type_store);
                }
            }
            Type::Function(func_type) => {
                for var in &[func_type.from, func_type.to] {
                    vars.insert(*var);
                    indices.insert(type_store.get_index(var));
                    let ty = type_store.get_type(var);
                    ty.collect(vars, args, indices, constraints, type_store);
                }
            }
            Type::TypeArgument(index, type_constraints) => {
                for c in type_constraints {
                    constraints.add(*index, *c);
                }
                args.insert(*index);
            }
            Type::FixedTypeArgument(index, _, type_constraints) => {
                for c in type_constraints {
                    constraints.add(*index, *c);
                }
                args.insert(*index);
            }
            Type::Named(_, _, type_vars) => {
                for var in type_vars {
                    vars.insert(*var);
                    indices.insert(type_store.get_index(var));
                    let ty = type_store.get_type(var);
                    ty.collect(vars, args, indices, constraints, type_store);
                }
            }
        }
    }

    pub fn as_string(
        &self,
        type_store: &TypeStore,
        need_parens: bool,
        type_args: &BTreeMap<usize, String>,
    ) -> String {
        match self {
            Type::Nothing => format!("!"),
            Type::Tuple(type_vars) => {
                let ss: Vec<_> = type_vars
                    .iter()
                    .map(|var| {
                        let ty = type_store.get_type(var);
                        ty.as_string(type_store, false, type_args)
                    })
                    .collect();
                format!("({})", ss.join(", "))
            }
            Type::Function(func_type) => {
                let func_type_str = func_type.as_string(type_store, type_args);
                if need_parens {
                    format!("({})", func_type_str)
                } else {
                    func_type_str
                }
            }
            Type::TypeArgument(index, _) => format!(
                "{}",
                type_args.get(index).expect("readable type arg not found")
            ),
            Type::FixedTypeArgument(_, name, _) => format!("{}", name),
            Type::Named(name, _, type_vars) => {
                let ss: Vec<_> = type_vars
                    .iter()
                    .map(|var| {
                        let ty = type_store.get_type(var);
                        ty.as_string(type_store, false, type_args)
                    })
                    .collect();
                format!("{} {}", name, ss.join(" "))
            }
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
            Type::Named(_, _, type_vars) => {
                for var in type_vars {
                    if check_sub_var(var, vars, type_store) {
                        return true;
                    }
                }
                false
            }
        }
    }

    pub fn debug_dump(&self, type_store: &TypeStore) -> String {
        match self {
            Type::Nothing => format!("!"),
            Type::Tuple(type_vars) => {
                let ss: Vec<_> = type_vars
                    .iter()
                    .map(|var| type_store.debug_var(var))
                    .collect();
                format!("({})", ss.join(", "))
            }
            Type::Function(func_type) => {
                let from = type_store.debug_var(&func_type.from);
                let to = type_store.debug_var(&func_type.to);
                format!("{} -> {}", from, to)
            }
            Type::TypeArgument(index, _) => format!("type_arg{}", index),
            Type::FixedTypeArgument(index, n, _) => format!("fixed_type_arg({}){}", n, index),
            Type::Named(n, id, type_vars) => {
                let ss: Vec<_> = type_vars
                    .iter()
                    .map(|var| type_store.debug_var(var))
                    .collect();
                format!("named({},{}({}))", n, id, ss.join(", "))
            }
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Nothing => write!(f, "!"),
            Type::Tuple(types) => {
                let ss: Vec<_> = types.iter().map(|t| format!("{}", t)).collect();
                write!(f, "({})", ss.join(", "))
            }
            Type::Function(func_type) => write!(f, "{}", func_type),
            Type::TypeArgument(index, _) => write!(f, "t{}", index),
            Type::FixedTypeArgument(_, name, _) => write!(f, "{}", name),
            Type::Named(name, _, types) => {
                let ss: Vec<_> = types.iter().map(|t| format!("{}", t)).collect();
                write!(f, "{} {}", name, ss.join(" "))
            }
        }
    }
}
