use super::type_variable::TypeVariable;
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

    pub fn get_unique_type_arg_type(&mut self) -> Type {
        let ty = Type::TypeArgument {
            index: self.arg_counter.next(),
            user_defined: false,
        };
        ty
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

    fn merge(&mut self, from: &TypeVariable, to: &TypeVariable) {
        let from_index = self.get_index(from);
        let to_index = self.get_index(to);
        for (_, value) in self.variables.iter_mut() {
            if *value == to_index {
                *value = from_index;
            }
        }
    }

    fn set_variable_type(&mut self, var: &TypeVariable, ty: Type) {
        let index = self.get_index(var);
        self.indices.insert(index, ty);
    }

    pub fn unify(
        &mut self,
        var1: &TypeVariable,
        var2: &TypeVariable,
        unified_variables: &mut bool,
    ) -> bool {
        let var_ty1 = self.get_type(var1);
        let var_ty2 = self.get_type(var2);
        /*
        println!(
            "Unify vars t1:({}),{:?} t2:({}),{:?}",
            var_ty1, var1, var_ty2, var2
        );
        */
        let index1 = self.get_index(var1);
        let index2 = self.get_index(var2);
        if index1 == index2 {
            return true;
        }
        match (&var_ty1, &var_ty2) {
            (Type::Int, Type::Int) => {}
            (Type::String, Type::String) => {}
            (Type::Bool, Type::Bool) => {}
            (
                Type::TypeArgument {
                    index: _,
                    user_defined: false,
                },
                _,
            ) => {
                *unified_variables = true;
                self.merge(var2, var1);
            }
            (
                _,
                Type::TypeArgument {
                    index: _,
                    user_defined: false,
                },
            ) => {
                *unified_variables = true;
                self.merge(var1, var2);
            }
            (Type::Tuple(type_vars1), Type::Tuple(type_vars2)) => {
                if type_vars1.len() != type_vars2.len() {
                    return false;
                } else {
                    for (v1, v2) in type_vars1.iter().zip(type_vars2.iter()) {
                        if !self.unify(v1, v2, unified_variables) {
                            return false;
                        }
                    }
                }
            }
            (Type::Function(f1), Type::Function(f2)) => {
                if !self.unify(&f1.from, &f2.from, unified_variables) {
                    return false;
                }
                if !self.unify(&f1.to, &f2.to, unified_variables) {
                    return false;
                }
            }
            _ => {
                return false;
            }
        }
        return true;
    }

    pub fn unify_variable_with_type(
        &mut self,
        var: &TypeVariable,
        ty: &Type,
        unified_variables: &mut bool,
    ) -> bool {
        let var_ty = self.get_type(var);
        match (&var_ty, &ty) {
            (Type::Int, Type::Int) => {}
            (Type::String, Type::String) => {}
            (Type::Bool, Type::Bool) => {}
            (
                _,
                Type::TypeArgument {
                    index: _,
                    user_defined: false,
                },
            ) => unreachable!(),
            (
                Type::TypeArgument {
                    index: _,
                    user_defined: false,
                },
                _,
            ) => {
                *unified_variables = true;
                self.set_variable_type(var, ty.clone());
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
        ty.as_string(self, false)
    }

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
        for (var, _) in &self.variables {
            println!("{} => {}", var, self.get_resolved_type_string(var));
        }
    }
}
