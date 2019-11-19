use crate::class::ClassId;
use crate::data::TypeDefId;
use crate::program::Program;
use crate::type_var_generator::TypeVarGenerator;
use crate::unifier::Unifier;
use siko_util::format_list;
use siko_util::Collector;
use siko_util::Counter;
use std::collections::BTreeMap;
use std::fmt;

pub struct ResolverContext {
    type_args: BTreeMap<usize, String>,
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
            self.type_args.insert(arg, format!("t{}", index));
        }
    }

    pub fn add_named_type_arg(&mut self, arg: usize, name: String) {
        if !self.type_args.contains_key(&arg) {
            self.type_args.insert(arg, name);
        }
    }

    pub fn get_type_arg_name(&self, arg: usize) -> String {
        self.type_args
            .get(&arg)
            .expect("type arg name not found")
            .clone()
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

    pub fn duplicate(
        &self,
        arg_map: &mut BTreeMap<usize, usize>,
        type_var_generator: &mut TypeVarGenerator,
    ) -> Type {
        match self {
            Type::Tuple(items) => {
                let new_items: Vec<_> = items
                    .iter()
                    .map(|i| i.duplicate(arg_map, type_var_generator))
                    .collect();
                Type::Tuple(new_items)
            }
            Type::Named(name, id, items) => {
                let new_items: Vec<_> = items
                    .iter()
                    .map(|i| i.duplicate(arg_map, type_var_generator))
                    .collect();
                Type::Named(name.clone(), *id, new_items)
            }
            Type::Function(from, to) => {
                let from = from.duplicate(arg_map, type_var_generator);
                let to = to.duplicate(arg_map, type_var_generator);
                Type::Function(Box::new(from), Box::new(to))
            }
            Type::Var(index, constraints) => {
                let mut gen = type_var_generator.clone();
                let new_index = arg_map.entry(*index).or_insert_with(|| gen.get_new_index());
                Type::Var(*new_index, constraints.clone())
            }
            Type::FixedTypeArg(name, index, constraints) => {
                let mut gen = type_var_generator.clone();
                let new_index = arg_map.entry(*index).or_insert_with(|| gen.get_new_index());
                Type::FixedTypeArg(name.clone(), *new_index, constraints.clone())
            }
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

    pub fn get_args(&self, args: &mut Vec<Type>) {
        match self {
            Type::Tuple(..) => {}
            Type::Named(..) => {}
            Type::Function(from, to) => {
                args.push(*from.clone());
                to.get_args(args);
            }
            Type::Var(..) => {}
            Type::FixedTypeArg(..) => {}
        }
    }

    pub fn get_result_type(&self, arg_count: usize) -> Type {
        match self {
            Type::Tuple(..) => self.clone(),
            Type::Named(..) => self.clone(),
            Type::Function(_, to) => {
                if arg_count == 1 {
                    *to.clone()
                } else {
                    if arg_count == 0 {
                        self.clone()
                    } else {
                        to.get_result_type(arg_count - 1)
                    }
                }
            }
            Type::Var(..) => self.clone(),
            Type::FixedTypeArg(..) => self.clone(),
        }
    }

    fn collect(&self, args: &mut Collector<usize, ClassId>, context: &mut ResolverContext) {
        match self {
            Type::Tuple(items) => {
                for item in items {
                    item.collect(args, context);
                }
            }
            Type::Named(_, _, items) => {
                for item in items {
                    item.collect(args, context);
                }
            }
            Type::Function(from, to) => {
                from.collect(args, context);
                to.collect(args, context);
            }
            Type::Var(index, constraints) => {
                args.add_empty(*index);
                for c in constraints {
                    args.add(*index, *c);
                }
            }
            Type::FixedTypeArg(name, index, constraints) => {
                args.add_empty(*index);
                context.add_named_type_arg(*index, name.clone());
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
        self.collect(&mut args, resolver_context);
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

    pub fn apply(&mut self, unifier: &Unifier) -> bool {
        let new = unifier.apply(self);
        let changed = *self != new;
        *self = new;
        changed
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
