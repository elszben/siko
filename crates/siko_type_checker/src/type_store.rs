use crate::check_context::CheckContext;
use crate::instance_resolver::ResolutionResult;
use crate::type_variable::TypeVariable;
use crate::types::Type;
use siko_ir::class::ClassId;
use siko_ir::types::ConcreteType;
use siko_ir::types::TypeDefId;
use siko_util::format_list;
use siko_util::Collector;
use siko_util::Counter;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::rc::Rc;

pub struct ResolverContext {
    type_args: BTreeMap<usize, String>,
    next_char: u32,
    list_type_id: Option<TypeDefId>,
}

impl ResolverContext {
    pub fn new() -> ResolverContext {
        ResolverContext {
            type_args: BTreeMap::new(),
            next_char: 'a' as u32,
            list_type_id: None,
        }
    }

    pub fn add_type_arg(&mut self, arg: usize) {
        if !self.type_args.contains_key(&arg) {
            let c = std::char::from_u32(self.next_char).expect("Invalid char");
            self.next_char += 1;
            let name = format!("{}", c);
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
        self.list_type_id.expect("list type id is not set")
    }
}

pub struct CloneContext<'a> {
    vars: BTreeMap<TypeVariable, TypeVariable>,
    args: BTreeMap<usize, usize>,
    indices: BTreeMap<TypeIndex, TypeIndex>,
    pub type_store: &'a mut TypeStore,
}

impl<'a> CloneContext<'a> {
    pub fn new(type_store: &'a mut TypeStore) -> CloneContext<'a> {
        CloneContext {
            vars: BTreeMap::new(),
            args: BTreeMap::new(),
            indices: BTreeMap::new(),
            type_store: type_store,
        }
    }

    pub fn var(&mut self, var: TypeVariable) -> TypeVariable {
        let r = CloneContext::build_var(self.type_store, &mut self.vars, var);
        r
    }

    pub fn arg(&mut self, arg: usize) -> usize {
        let r = CloneContext::build_arg(self.type_store, &mut self.args, arg);
        r
    }

    pub fn index(&mut self, index: TypeIndex) -> TypeIndex {
        let r = CloneContext::build_index(self.type_store, &mut self.indices, index);
        r
    }

    pub fn clone_var(&mut self, var: TypeVariable) -> TypeVariable {
        match self.vars.get(&var) {
            Some(v) => *v,
            None => {
                let new_var = self.var(var);
                let index = self.type_store.get_index(&var);
                let index = match self.indices.get(&index) {
                    Some(i) => *i,
                    None => {
                        let new_index = self.index(index);
                        let ty = self.type_store.indices[index.id].clone().unwrap();
                        let new_ty = ty.clone_type(self);
                        self.type_store.indices[new_index.id] = Some(new_ty);
                        new_index
                    }
                };
                self.type_store.variables[new_var.id] = Some(index);
                new_var
            }
        }
    }

    fn build_var(
        type_store: &mut TypeStore,
        vars: &mut BTreeMap<TypeVariable, TypeVariable>,
        var: TypeVariable,
    ) -> TypeVariable {
        let new_var = vars.entry(var).or_insert_with(|| type_store.allocate_var());
        *new_var
    }

    fn build_arg(
        type_store: &mut TypeStore,
        args: &mut BTreeMap<usize, usize>,
        arg: usize,
    ) -> usize {
        let new_arg = args
            .entry(arg)
            .or_insert_with(|| type_store.get_unique_type_arg());
        *new_arg
    }

    fn build_index(
        type_store: &mut TypeStore,
        indices: &mut BTreeMap<TypeIndex, TypeIndex>,
        index: TypeIndex,
    ) -> TypeIndex {
        let new_index = indices
            .entry(index)
            .or_insert_with(|| type_store.allocate_index());
        *new_index
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeIndex {
    pub id: usize,
}

impl TypeIndex {
    pub fn new(id: usize) -> TypeIndex {
        TypeIndex { id: id }
    }
}

pub struct TypeStore {
    variables: Vec<Option<TypeIndex>>,
    indices: Vec<Option<Type>>,
    arg_counter: Counter,
    check_context: Rc<RefCell<CheckContext>>,
    list_type_id: TypeDefId,
}

impl TypeStore {
    pub fn new(list_type_id: TypeDefId, check_context: Rc<RefCell<CheckContext>>) -> TypeStore {
        TypeStore {
            variables: Vec::new(),
            indices: Vec::new(),
            arg_counter: Counter::new(),
            check_context: check_context,
            list_type_id: list_type_id,
        }
    }

    pub fn get_unique_type_arg(&mut self) -> usize {
        self.arg_counter.next()
    }

    pub fn get_unique_type_arg_type(&mut self) -> Type {
        let ty = Type::TypeArgument(self.arg_counter.next(), vec![]);
        ty
    }

    fn allocate_var(&mut self) -> TypeVariable {
        let var = self.variables.len();
        self.variables.push(None);
        let type_var = TypeVariable::new(var);
        type_var
    }

    fn allocate_index(&mut self) -> TypeIndex {
        let index = self.indices.len();
        self.indices.push(None);
        let index = TypeIndex::new(index);
        index
    }

    pub fn add_type(&mut self, ty: Type) -> TypeVariable {
        let type_var = self.allocate_var();
        let index = self.allocate_index();
        self.indices[index.id] = Some(ty);
        self.variables[type_var.id] = Some(index);
        type_var
    }

    pub fn add_var_and_type(&mut self, type_var: TypeVariable, ty: Type, index: TypeIndex) {
        self.indices[index.id] = Some(ty);
        self.variables[type_var.id] = Some(index);
    }

    pub fn get_index(&self, var: &TypeVariable) -> TypeIndex {
        self.variables[var.id].clone().unwrap()
    }

    pub fn merge(&mut self, from: &TypeVariable, to: &TypeVariable) {
        let from_index = self.get_index(from);
        let to_index = self.get_index(to);
        for value in self.variables.iter_mut() {
            if let Some(value) = value {
                if *value == to_index {
                    *value = from_index;
                }
            }
        }
    }

    pub fn has_class_instance(
        &mut self,
        var: &TypeVariable,
        class_id: &ClassId,
    ) -> ResolutionResult {
        let concrete_type = self.to_concrete_type(var);
        let context = self.check_context.clone();
        let context = context.borrow();
        /*println!(
            "Resolving instance for type: {}, {:?}",
            self.get_resolved_type_string(var),
            concrete_type
        );*/
        context.instance_resolver.has_class_instance(
            var,
            class_id,
            self,
            context.type_instance_resolver.clone(),
            concrete_type,
        )
    }

    pub fn unify(&mut self, primary: &TypeVariable, secondary: &TypeVariable) -> bool {
        let primary_type = self.get_type(primary);
        let secondary_type = self.get_type(secondary);

        let index1 = self.get_index(primary);
        let index2 = self.get_index(secondary);
        /*
        println!(
            "Unify vars t1:({}),{:?},{} t2:({}),{:?},{}",
            primary, primary_type, index1.id, secondary, secondary_type, index2.id,
        );
        */
        if index1 == index2 {
            return true;
        }
        match (&primary_type, &secondary_type) {
            (
                Type::TypeArgument(_, primary_constraints),
                Type::TypeArgument(_, secondary_constraints),
            ) => {
                let mut merged_constraints = primary_constraints.clone();
                merged_constraints.extend(secondary_constraints);
                merged_constraints.sort();
                merged_constraints.dedup();
                let merged_type =
                    Type::TypeArgument(self.get_unique_type_arg(), merged_constraints);
                let merged_type_var = self.add_type(merged_type);
                self.merge(&merged_type_var, primary);
                self.merge(&merged_type_var, secondary);
            }
            (
                Type::TypeArgument(_, primary_constraints),
                Type::FixedTypeArgument(_, _, secondary_constraints),
            ) => {
                for c in primary_constraints {
                    if !secondary_constraints.contains(c) {
                        return false;
                    }
                }
                self.merge(secondary, primary);
            }
            (
                Type::FixedTypeArgument(_, _, primary_constraints),
                Type::TypeArgument(_, secondary_constraints),
            ) => {
                for c in secondary_constraints {
                    if !primary_constraints.contains(c) {
                        return false;
                    }
                }
                self.merge(primary, secondary);
            }
            (Type::TypeArgument(_, constraints), _) => {
                for c in constraints {
                    if self.has_class_instance(secondary, c) == ResolutionResult::No {
                        return false;
                    }
                }
                self.merge(secondary, primary);
            }
            (_, Type::TypeArgument(_, constraints)) => {
                for c in constraints {
                    if self.has_class_instance(primary, c) == ResolutionResult::No {
                        return false;
                    }
                }
                self.merge(primary, secondary);
            }
            (Type::Tuple(type_vars1), Type::Tuple(type_vars2)) => {
                if type_vars1.len() != type_vars2.len() {
                    return false;
                } else {
                    for (v1, v2) in type_vars1.iter().zip(type_vars2.iter()) {
                        if !self.unify(v1, v2) {
                            return false;
                        }
                    }
                }
            }
            (Type::Function(f1), Type::Function(f2)) => {
                if !self.unify(&f1.from, &f2.from) {
                    return false;
                }
                if !self.unify(&f1.to, &f2.to) {
                    return false;
                }
            }
            (Type::Named(_, id1, type_vars1), Type::Named(_, id2, type_vars2)) => {
                if id1 != id2 {
                    return false;
                }
                if type_vars1.len() != type_vars2.len() {
                    return false;
                } else {
                    for (v1, v2) in type_vars1.iter().zip(type_vars2.iter()) {
                        if !self.unify(v1, v2) {
                            return false;
                        }
                    }
                }
            }
            _ => {
                return false;
            }
        }
        return true;
    }

    pub fn get_type(&self, var: &TypeVariable) -> Type {
        let index = self.get_index(var);
        self.indices[index.id].clone().unwrap()
    }

    pub fn to_concrete_type(&self, var: &TypeVariable) -> Option<ConcreteType> {
        let ty = self.get_type(var);
        ty.to_concrete_type(self)
    }

    pub fn get_type_args(&self, var: &TypeVariable) -> BTreeSet<usize> {
        let ty = self.get_type(var);
        let mut vars = BTreeSet::new();
        let mut args = BTreeSet::new();
        let mut constraints = Collector::new();
        ty.collect(&mut vars, &mut args, &mut constraints, self);
        args
    }

    pub fn get_resolved_type_string(&self, var: &TypeVariable) -> String {
        let mut resolver_context = ResolverContext::new();
        self.get_resolved_type_string_with_context(var, &mut resolver_context)
    }

    pub fn get_resolved_type_string_with_context(
        &self,
        var: &TypeVariable,
        resolver_context: &mut ResolverContext,
    ) -> String {
        if self.is_recursive(*var) {
            return format!("<recursive type>");
        }
        resolver_context.list_type_id = Some(self.list_type_id);
        let ty = self.get_type(var);
        let mut vars = BTreeSet::new();
        let mut args = BTreeSet::new();
        let mut constraints = Collector::new();
        ty.collect(&mut vars, &mut args, &mut constraints, self);
        for arg in args {
            resolver_context.add_type_arg(arg);
        }
        let mut constraint_strings = Vec::new();
        for (c, classes) in constraints.items {
            for class_id in classes {
                let context = self.check_context.borrow();
                let class_name = context
                    .class_names
                    .get(&class_id)
                    .expect("Class name not found");
                let c_str = format!("{} {}", class_name, resolver_context.get_type_arg_name(c));
                constraint_strings.push(c_str);
            }
        }

        let prefix = if constraint_strings.is_empty() {
            format!("")
        } else {
            format!("({}) => ", format_list(&constraint_strings[..]))
        };
        let type_str = ty.as_string(self, false, resolver_context);
        format!("{}{}", prefix, type_str)
    }

    pub fn get_new_type_var(&mut self) -> TypeVariable {
        let ty = self.get_unique_type_arg_type();
        let var = self.add_type(ty);
        var
    }

    pub fn create_clone_context(&mut self) -> CloneContext {
        CloneContext::new(self)
    }

    pub fn clone_type_var_simple(&mut self, var: TypeVariable) -> TypeVariable {
        let mut context = self.create_clone_context();
        let ty = context.type_store.get_type(&var);
        let new_ty = ty.clone_type(&mut context);
        context.type_store.add_type(new_ty)
    }

    pub fn is_recursive(&self, var: TypeVariable) -> bool {
        let ty = self.get_type(&var);
        let vars = vec![var];
        ty.check_recursion(&vars, self)
    }

    #[allow(unused)]
    pub fn debug_var(&self, var: &TypeVariable) -> String {
        let ty = self.get_type(var);
        format!(
            "{}:{}({})",
            var,
            self.get_index(var).id,
            ty.debug_dump(self)
        )
    }
}
