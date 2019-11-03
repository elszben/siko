use crate::typechecker::TypeVarGenerator;
use siko_ir::class::ClassId;
use siko_ir::types::TypeDefId;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum BaseType {
    Tuple,
    Named(TypeDefId),
    Function,
    Generic,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Type {
    Tuple(Vec<Type>),
    Named(String, TypeDefId, Vec<Type>),
    Function(Box<Type>, Box<Type>),
    Var(usize, Vec<ClassId>),
    FixedTypeArg(String, usize, Vec<ClassId>),
}

impl Type {
    pub fn contains(&self, index: usize) -> bool {
        match self {
            Type::Tuple(items) => {
                for item in items {
                    if item.contains(index) {
                        return true;
                    }
                }
                return false;
            }
            Type::Named(_, _, items) => {
                for item in items {
                    if item.contains(index) {
                        return true;
                    }
                }
                return false;
            }
            Type::Function(from, to) => {
                if from.contains(index) {
                    return true;
                }
                if to.contains(index) {
                    return true;
                }
                return false;
            }
            Type::Var(i, _) => {
                return *i == index;
            }
            Type::FixedTypeArg(_, i, _) => {
                return *i == index;
            }
        }
    }

    pub fn add_constraints(&self, constraints: &Vec<ClassId>) -> Type {
        match self {
            Type::Var(index, cs) => {
                let mut cs = cs.clone();
                cs.extend(constraints);
                Type::Var(*index, cs)
            }
            _ => unreachable!(),
        }
    }

    pub fn get_base_type(&self) -> BaseType {
        match self {
            Type::Tuple(..) => BaseType::Tuple,
            Type::Named(_, id, _) => BaseType::Named(*id),
            Type::Function(..) => BaseType::Function,
            Type::Var(..) => BaseType::Generic,
            Type::FixedTypeArg(..) => BaseType::Generic,
        }
    }

    pub fn remove_fixed_types(&self) -> Type {
        match self {
            Type::Tuple(items) => {
                let new_items: Vec<_> = items.iter().map(|i| i.remove_fixed_types()).collect();
                Type::Tuple(new_items)
            }
            Type::Named(name, id, items) => {
                let new_items: Vec<_> = items.iter().map(|i| i.remove_fixed_types()).collect();
                Type::Named(name.clone(), *id, new_items)
            }
            Type::Function(from, to) => {
                let from = from.remove_fixed_types();
                let to = to.remove_fixed_types();
                Type::Function(Box::new(from), Box::new(to))
            }
            Type::Var(..) => self.clone(),
            Type::FixedTypeArg(_, index, constraints) => Type::Var(*index, constraints.clone()),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Tuple(items) => {
                let ss: Vec<_> = items.iter().map(|i| format!("{}", i)).collect();
                write!(f, "({})", ss.join(", "))
            }
            Type::Named(name, _, items) => {
                let ss: Vec<_> = items.iter().map(|i| format!("{}", i)).collect();
                let args = if ss.is_empty() {
                    "".to_string()
                } else {
                    format!(" ({})", ss.join(" "))
                };
                write!(f, "{}{}", name, args)
            }
            Type::Function(from, to) => write!(f, "{} -> {}", from, to),
            Type::Var(id, constraints) => {
                let c = if constraints.is_empty() {
                    format!("")
                } else {
                    format!(
                        "/{}",
                        constraints
                            .iter()
                            .map(|c| format!("{}", c))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                write!(f, "${}{}", id, c)
            }
            Type::FixedTypeArg(_, id, constraints) => {
                let c = if constraints.is_empty() {
                    format!("")
                } else {
                    format!(
                        "/{}",
                        constraints
                            .iter()
                            .map(|c| format!("{}", c))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                write!(f, "f${}{}", id, c)
            }
        }
    }
}
