use super::function_type::FunctionType;
use super::function_type::FunctionTypeStore;
use super::type_store::TypeStore;
use super::type_store::UnificationResultCollector;
use super::type_variable::TypeVariable;
use super::types::Type;
use crate::error::Error;
use crate::ir::FunctionId;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct FunctionHeader {
    pub function_type: Option<FunctionType>,
    pub arguments: Vec<TypeVariable>,
    pub body_return_type: Option<TypeVariable>,
    pub used_functions: Vec<FunctionId>,
}

impl FunctionHeader {
    pub fn check(&mut self, type_store: &TypeStore) -> Result<(), Error> {
        let body_return_type_var = match self.body_return_type {
            Some(v) => v.clone(),
            None => {
                return Ok(());
            }
        };
        let mut types: Vec<_> = self
            .arguments
            .iter()
            .map(|v| type_store.get_resolved_type(v.clone()))
            .collect();
        types.push(type_store.get_resolved_type(body_return_type_var));
        let inferred_type = FunctionType::new(types);
        println!("Header inferred type: {}", inferred_type);
        if let Some(ft) = &self.function_type {
            if *ft != inferred_type {
                let err = format!(
                    "function signature does not match inferred type {} != {}",
                    ft, inferred_type
                );
                let err = Error::typecheck_err(err);
                return Err(err);
            }
        } else {
            self.function_type = Some(inferred_type);
        }
        Ok(())
    }
}

fn convert_type_into_vars(
    ty: &Type,
    type_store: &mut TypeStore,
    arg_mapping: &mut BTreeMap<usize, TypeVariable>,
) -> TypeVariable {
    match ty {
        Type::Int => type_store.add_var(ty.clone()),
        Type::Bool => type_store.add_var(ty.clone()),
        Type::String => type_store.add_var(ty.clone()),
        Type::Nothing => type_store.add_var(ty.clone()),
        Type::Tuple(types) => {
            let vars = types
                .iter()
                .map(|t| convert_type_into_vars(t, type_store, arg_mapping))
                .map(|v| Type::TypeVar(v))
                .collect();
            type_store.add_var(Type::Tuple(vars))
        }
        Type::Function(func_type) => {
            let vars = func_type
                .types
                .iter()
                .map(|t| convert_type_into_vars(t, type_store, arg_mapping))
                .map(|v| Type::TypeVar(v))
                .collect();
            type_store.add_var(Type::Function(FunctionType::new(vars)))
        }
        Type::TypeArgument(arg) => {
            let var = arg_mapping.entry(*arg).or_insert_with(|| {
                let arg_id = type_store.get_unique_type_arg();
                type_store.add_var(Type::TypeArgument(arg_id))
            });
            *var
        }
        Type::TypeVar(_) => panic!("TypeVar in normal form type"),
    }
}

#[derive(Debug)]
pub struct FunctionCall {
    pub function_id: FunctionId,
    pub args: Vec<TypeVariable>,
    pub return_type: TypeVariable,
    pub callee_vars: Vec<TypeVariable>,
}

impl FunctionCall {
    pub fn prepare(&mut self, type_store: &mut TypeStore, function_type_store: &FunctionTypeStore) {
        println!("Checking function call for {:?}", self.function_id);
        let func_type = function_type_store
            .get_function_type(&self.function_id)
            .expect("Function type unavailable during function call constraint check");
        let mut type_arg_mapping = BTreeMap::new();
        self.callee_vars = func_type
            .types
            .iter()
            .map(|ty| convert_type_into_vars(&ty, type_store, &mut type_arg_mapping))
            .collect();
    }

    pub fn check(&mut self, type_store: &mut TypeStore, result: &mut UnificationResultCollector) {
        if self.args.len() >= self.callee_vars.len() {
            result.add_error(format!("Too many arguments"));
            return;
        }
        println!("Checking function call for {:?}", self.function_id);
        for (caller_var, callee_var) in self.args.iter().zip(self.callee_vars.iter()) {
            type_store.unify_vars(*callee_var, *caller_var, result);
        }
        let result_vars = &self.callee_vars[self.args.len()..];
        if result_vars.len() == 1 {
            type_store.unify_vars(self.return_type, result_vars[0], result);
        } else {
            let types = result_vars
                .iter()
                .map(|v| Type::TypeVar(v.clone()))
                .collect();
            let result_func_type = FunctionType::new(types);
            let result_func_type = Type::Function(result_func_type);
            let result_var = type_store.add_var(result_func_type);
            type_store.unify_vars(self.return_type, result_var, result);
        }
    }
}

#[derive(Debug)]
pub enum TypeConstraint {
    FunctionCall(FunctionCall),
}

impl TypeConstraint {
    pub fn prepare(&mut self, type_store: &mut TypeStore, function_type_store: &FunctionTypeStore) {
        match self {
            TypeConstraint::FunctionCall(call) => {
                call.prepare(type_store, function_type_store);
            }
        }
    }
    pub fn check(&mut self, type_store: &mut TypeStore, result: &mut UnificationResultCollector) {
        match self {
            TypeConstraint::FunctionCall(call) => {
                call.check(type_store, result);
            }
        }
    }
}
