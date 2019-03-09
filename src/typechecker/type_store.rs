use super::type_variable::TypeVariable;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::types::Type;
use crate::util::Counter;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeIndex {
    id: usize,
}

impl TypeIndex {
    pub fn new(id: usize) -> TypeIndex {
        TypeIndex { id: id }
    }
}

#[derive(Debug)]
pub struct TypeStore {
    variables: BTreeMap<TypeVariable, TypeIndex>,
    indices: BTreeMap<TypeIndex, Type>,
    var_counter: Counter,
    index_counter: Counter,
    arg_counter: Counter,
}

impl TypeStore {
    pub fn new() -> TypeStore {
        TypeStore {
            variables: BTreeMap::new(),
            indices: BTreeMap::new(),
            var_counter: Counter::new(),
            index_counter: Counter::new(),
            arg_counter: Counter::new(),
        }
    }

    pub fn get_unique_type_arg(&mut self) -> usize {
        self.arg_counter.next()
    }

    fn allocate_var(&mut self) -> TypeVariable {
        let type_var = TypeVariable::new(self.var_counter.next());
        type_var
    }

    fn allocate_index(&mut self) -> TypeIndex {
        let index = TypeIndex::new(self.index_counter.next());
        index
    }

    pub fn add_var(&mut self, ty: Type) -> TypeVariable {
        let type_var = self.allocate_var();
        let index = self.allocate_index();
        self.indices.insert(index, ty);
        self.variables.insert(type_var, index);
        type_var
    }

    pub fn add_var_and_type(&mut self, type_var: TypeVariable, ty: Type) {
        let index = self.allocate_index();
        self.indices.insert(index, ty);
        self.variables.insert(type_var, index);
    }

    fn get_index(&self, var: &TypeVariable) -> TypeIndex {
        self.variables
            .get(var)
            .expect("invalid type variable")
            .clone()
    }

    fn unify_variables(&mut self, from: &TypeVariable, to: &TypeVariable) {
        let from_index = self.get_index(from);
        let to_index = self.get_index(to);
        for (_, value) in self.variables.iter_mut() {
            if *value == to_index {
                *value = from_index;
            }
        }
    }

    pub fn unify_vars(&mut self, var1: &TypeVariable, var2: &TypeVariable) -> bool {
        let var_ty1 = self.get_type(var1);
        let var_ty2 = self.get_type(var2);
        println!(
            "Unify vars t1:{},{:?} t2:{},{:?}",
            var_ty1, var1, var_ty2, var2
        );

        match (&var_ty1, &var_ty2) {
            (Type::Int, Type::Int) => {}
            (Type::String, Type::String) => {}
            (Type::Bool, Type::Bool) => {}
            (Type::TypeArgument(_), _) => {
                let indx1 = self.get_index(&var1);
                self.unify_variables(var2, var1);
                let indx2 = self.get_index(&var1);
                println!(
                    "indx1 {:?}, indx2 {:?} {}",
                    indx1,
                    indx2,
                    self.get_type(&var1)
                );
            }
            (_, Type::TypeArgument(_)) => {
                self.unify_variables(var1, var2);
            }
            (Type::Tuple(type_vars1), Type::Tuple(type_vars2)) => {
                if type_vars1.len() != type_vars2.len() {
                    return false;
                } else {
                    for (v1, v2) in type_vars1.iter().zip(type_vars2.iter()) {
                        if !self.unify_vars(v1, v2) {
                            return false;
                        }
                    }
                }
            }
            (Type::Function(f1), Type::Function(f2)) => {
                if f1.type_vars.len() != f2.type_vars.len() {
                    return false;
                } else {
                    for (v1, v2) in f1.type_vars.iter().zip(f2.type_vars.iter()) {
                        if !self.unify_vars(v1, v2) {
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
        self.indices
            .get(&index)
            .expect("invalid type index")
            .clone()
    }

    pub fn get_resolved_type_string(&self, var: &TypeVariable) -> String {
        let ty = self.get_type(var);
        ty.as_string(self)
    }

    /*
        pub fn get_resolved_type(&self, var: &TypeVariable) -> Type {
            let index = self.get_index(var);
            let t = self
                .indices
                .get(&index)
                .expect("invalid type index")
                .clone();
            match t {
                Type::Tuple(inners) => {
                    let resolved_types = inners
                        .into_iter()
                        .map(|v| match v {
                            Type::TypeVar(v) => self.get_resolved_type(&v),
                            _ => v,
                        })
                        .collect();
                    return Type::Tuple(resolved_types);
                }
                Type::TypeVar(inner) => {
                    return self.get_resolved_type(&inner);
                }
                Type::Function(inner) => {
                    let resolved_types = inner
                        .types
                        .into_iter()
                        .map(|v| match v {
                            Type::TypeVar(v) => self.get_resolved_type(&v),
                            _ => v,
                        })
                        .collect();
                    return Type::Function(FunctionType::new(resolved_types));
                }
                _ => {
                    return t;
                }
            }
        }
    */
    pub fn clone_type(&mut self, ty: &Type) -> Type {
        let mut vars = Vec::new();
        let mut args = Vec::new();
        ty.collect(&mut vars, &mut args, self);
        let mut var_map = BTreeMap::new();
        let mut arg_map = BTreeMap::new();
        for var in vars {
            var_map.insert(var, self.allocate_var());
        }
        for arg in args {
            arg_map.insert(arg, self.get_unique_type_arg());
        }
        ty.clone_type(&var_map, &arg_map, self)
    }

    pub fn dump(&self) {
        for (var, idx) in &self.variables {
            println!("{:?} => {:?}", var, idx);
        }
    }
}
