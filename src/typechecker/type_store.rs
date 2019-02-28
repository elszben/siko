use super::type_variable::TypeVariable;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::types::Type;
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
    next_type_arg_id: usize,
}

impl TypeStore {
    pub fn new() -> TypeStore {
        TypeStore {
            variables: BTreeMap::new(),
            indices: BTreeMap::new(),
            next_type_arg_id: 0,
        }
    }

    pub fn get_unique_type_arg(&mut self) -> usize {
        let id = self.next_type_arg_id;
        self.next_type_arg_id += 1;
        id
    }

    pub fn add_var(&mut self, ty: Type) -> TypeVariable {
        let type_var = TypeVariable::new(self.variables.len());
        let index = TypeIndex::new(self.indices.len());
        self.indices.insert(index, ty);
        self.variables.insert(type_var, index);
        type_var
    }

    fn get_index(&self, var: TypeVariable) -> TypeIndex {
        self.variables
            .get(&var)
            .expect("invalid type variable")
            .clone()
    }

    fn unify_variables(&mut self, from: TypeVariable, to: TypeVariable) {
        let from_index = self.get_index(from);
        let to_index = self.get_index(to);
        for (_, value) in self.variables.iter_mut() {
            if *value == to_index {
                *value = from_index;
            }
        }
    }

    pub fn unify_vars(&mut self, var1: TypeVariable, var2: TypeVariable) -> bool {
        let var_ty1 = self.get_type(var1);
        let var_ty2 = self.get_type(var2);
        println!("Unify vars t1:{} t2:{}", var_ty1, var_ty2);
        match (&var_ty1, &var_ty2) {
            (Type::Int, Type::Int) => {}
            (Type::String, Type::String) => {}
            (Type::Bool, Type::Bool) => {}
            (Type::TypeArgument(_), _) => {
                self.unify_variables(var2, var1);
            }
            (_, Type::TypeArgument(_)) => {
                self.unify_variables(var1, var2);
            }
            (Type::Tuple(subtypes1), Type::Tuple(subtypes2)) => {
                if subtypes1.len() != subtypes2.len() {
                    return false;
                } else {
                    for (v1, v2) in subtypes1.iter().zip(subtypes2.iter()) {
                        if !self.unify_vars(v1.get_inner_type_var(), v2.get_inner_type_var()) {
                            return false;
                        }
                    }
                }
            }
            (Type::Function(f1), Type::Function(f2)) => {
                if f1.types.len() != f2.types.len() {
                    return false;
                } else {
                    for (v1, v2) in f1.types.iter().zip(f2.types.iter()) {
                        if !self.unify_vars(v1.get_inner_type_var(), v2.get_inner_type_var()) {
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

    pub fn get_type(&self, var: TypeVariable) -> Type {
        let index = self.get_index(var);
        self.indices
            .get(&index)
            .expect("invalid type index")
            .clone()
    }

    pub fn get_resolved_type(&self, var: TypeVariable) -> Type {
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
                        Type::TypeVar(v) => self.get_resolved_type(v),
                        _ => v,
                    })
                    .collect();
                return Type::Tuple(resolved_types);
            }
            Type::TypeVar(inner) => {
                return self.get_resolved_type(inner);
            }
            Type::Function(inner) => {
                let resolved_types = inner
                    .types
                    .into_iter()
                    .map(|v| match v {
                        Type::TypeVar(v) => self.get_resolved_type(v),
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

    pub fn dump(&self) {
        for var in self.variables.keys() {
            let ty = self.get_type(*var);
            let index = self.get_index(*var);
            println!("{:?} {:?} -> {}", var, index, ty);
        }
    }
}
