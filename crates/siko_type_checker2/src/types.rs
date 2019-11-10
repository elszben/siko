use siko_ir::class::ClassId;
use siko_ir::program::Program;
use siko_ir::types::TypeDefId;
use siko_util::format_list;
use siko_util::Collector;
use siko_util::Counter;
use std::collections::BTreeMap;
use std::fmt;

pub struct ResolverContext {
    type_args: BTreeMap<usize, usize>,
    next_index: Counter,
    list_type_id: TypeDefId,
    class_names: BTreeMap<ClassId, String>,
}

impl ResolverContext {
    pub fn new(program: &Program) -> ResolverContext {
        let mut class_names = BTreeMap::new();
        for (name, class) in &program.class_names {
            class_names.insert(*class, name.clone());
        }
        ResolverContext {
            type_args: BTreeMap::new(),
            next_index: Counter::new(),
            list_type_id: program.get_named_type("Data.List", "List"),
            class_names: class_names,
        }
    }

    pub fn add_type_arg(&mut self, arg: usize) {
        if !self.type_args.contains_key(&arg) {
            let index = self.next_index.next();
            self.type_args.insert(arg, index);
        }
    }

    pub fn get_type_arg_name(&self, arg: usize) -> String {
        format!(
            "t{}",
            self.type_args.get(&arg).expect("type arg name not found")
        )
    }

    pub fn get_list_type_id(&self) -> TypeDefId {
        self.list_type_id
    }

    pub fn get_class_name(&self, class_id: &ClassId) -> &String {
        self.class_names
            .get(class_id)
            .expect("Class name not found")
    }
}

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

    pub fn get_arg_count(&self) -> usize {
        match self {
            Type::Tuple(..) => 0,
            Type::Named(..) => 0,
            Type::Function(_, to) => 1 + to.get_arg_count(),
            Type::Var(..) => 0,
            Type::FixedTypeArg(..) => 0,
        }
    }

    fn collect(&self, args: &mut Collector<usize, ClassId>) {
        match self {
            Type::Tuple(items) => {
                for item in items {
                    item.collect(args);
                }
            }
            Type::Named(_, _, items) => {
                for item in items {
                    item.collect(args);
                }
            }
            Type::Function(from, to) => {
                from.collect(args);
                to.collect(args);
            }
            Type::Var(index, constraints) => {
                args.add_empty(*index);
                for c in constraints {
                    args.add(*index, *c);
                }
            }
            Type::FixedTypeArg(_, index, constraints) => {
                args.add_empty(*index);
                for c in constraints {
                    args.add(*index, *c);
                }
            }
        }
    }

    pub fn get_resolved_type_string(&self, program: &Program) -> String {
        let mut resolver_context = ResolverContext::new(program);
        self.get_resolved_type_string_with_context(&mut resolver_context)
    }

    pub fn get_resolved_type_string_with_context(
        &self,
        resolver_context: &mut ResolverContext,
    ) -> String {
        let mut args = Collector::new();
        self.collect(&mut args);
        for (arg, _) in &args.items {
            resolver_context.add_type_arg(*arg);
        }
        let mut constraint_strings = Vec::new();
        for (arg, classes) in &args.items {
            for class_id in classes {
                let class_name = resolver_context.get_class_name(class_id);
                let c_str = format!(
                    "{} {}",
                    class_name,
                    resolver_context.get_type_arg_name(*arg)
                );
                constraint_strings.push(c_str);
            }
        }

        let prefix = if constraint_strings.is_empty() {
            format!("")
        } else {
            format!("({}) => ", format_list(&constraint_strings[..]))
        };
        let type_str = self.as_string(false, resolver_context);
        format!("{}{}", prefix, type_str)
    }

    fn as_string(&self, need_parens: bool, resolver_context: &ResolverContext) -> String {
        match self {
            Type::Tuple(items) => {
                let ss: Vec<_> = items
                    .iter()
                    .map(|item| item.as_string(false, resolver_context))
                    .collect();
                format!("({})", ss.join(", "))
            }
            Type::Function(from, to) => {
                let from_str = from.as_string(true, resolver_context);
                let to_str = to.as_string(true, resolver_context);
                let func_type_str = format!("{} -> {}", from_str, to_str);
                if need_parens {
                    format!("({})", func_type_str)
                } else {
                    func_type_str
                }
            }
            Type::Var(index, _) => resolver_context.get_type_arg_name(*index),
            Type::FixedTypeArg(name, _, _) => format!("{}", name),
            Type::Named(name, id, items) => {
                let ss: Vec<_> = items
                    .iter()
                    .map(|item| item.as_string(true, resolver_context))
                    .collect();
                if *id == resolver_context.get_list_type_id() {
                    assert_eq!(ss.len(), 1);
                    format!("[{}]", ss[0])
                } else {
                    let (args, simple) = if ss.is_empty() {
                        (format!(""), true)
                    } else {
                        (format!(" {}", ss.join(" ")), false)
                    };
                    if simple {
                        format!("{}{}", name, args)
                    } else {
                        if need_parens {
                            format!("({}{})", name, args)
                        } else {
                            format!("{}{}", name, args)
                        }
                    }
                }
            }
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
