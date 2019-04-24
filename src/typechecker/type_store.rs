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
        let ty = Type::TypeArgument(self.arg_counter.next());
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

    pub fn add_type(&mut self, ty: Type) -> TypeVariable {
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

    pub fn merge(&mut self, from: &TypeVariable, to: &TypeVariable) {
        let from_index = self.get_index(from);
        let to_index = self.get_index(to);
        for (_, value) in self.variables.iter_mut() {
            if *value == to_index {
                *value = from_index;
            }
        }
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
            (Type::Int, Type::Int) => {}
            (Type::String, Type::String) => {}
            (Type::Bool, Type::Bool) => {}
            (Type::TypeArgument(_), Type::TypeArgument(_)) => {
                self.merge(primary, secondary);
            }
            (Type::TypeArgument(_), _) => {
                self.merge(secondary, primary);
            }
            (_, Type::TypeArgument(_)) => {
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
        if self.is_recursive(*var) {
            return format!("<recursive type>");
        }
        let ty = self.get_type(var);
        let mut vars = Vec::new();
        let mut args = Vec::new();
        ty.collect(&mut vars, &mut args, self);
        let mut type_args = BTreeMap::new();
        let mut next_char = 'a' as u32;
        for arg in args {
            let c = std::char::from_u32(next_char).expect("Invalid char");
            type_args.insert(arg, format!("{}", c));
            next_char += 1;
        }
        ty.as_string(self, false, &type_args)
    }

    pub fn get_new_type_var(&mut self) -> TypeVariable {
        let ty = self.get_unique_type_arg_type();
        let var = self.add_type(ty);
        var
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

    pub fn clone_type_var(&mut self, var: TypeVariable) -> TypeVariable {
        let ty = self.get_type(&var);
        let new_ty = self.clone_type(&ty);
        self.add_type(new_ty)
    }

    pub fn is_recursive(&self, var: TypeVariable) -> bool {
        let ty = self.get_type(&var);
        let vars = vec![var];
        ty.check_recursion(&vars, self)
    }
}
