use crate::ir::function::FunctionId;
use crate::typechecker::types::Type;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FunctionType {
    pub types: Vec<Type>,
}

impl FunctionType {
    pub fn new(types: Vec<Type>) -> FunctionType {
        FunctionType { types: types }
    }

    pub fn get_return_type(&self) -> Type {
        self.types.last().expect("empty function!").clone()
    }

    pub fn get_arg_count(&self) -> usize {
        self.types.len() - 1
    }

    pub fn get_arg_types(&self) -> Vec<Type> {
        self.types[0..self.types.len() - 1].to_vec()
    }

    pub fn collect_type_arg(&self, used_type_args: &mut Vec<usize>) {
        for ty in &self.types {
            ty.collect_type_arg(used_type_args);
        }
        used_type_args.sort();
        used_type_args.dedup();
    }

    pub fn remap_type_args(&mut self, arg_mapping: &BTreeMap<usize, usize>) {
        for ty in &mut self.types {
            ty.remap_type_args(arg_mapping);
        }
    }
}

impl fmt::Display for FunctionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.get_arg_count() != 0 {
            let ss: Vec<_> = self.types.iter().map(|t| format!("{}", t)).collect();
            write!(f, "{}", ss.join(" -> "))
        } else {
            write!(f, "{}", self.get_return_type())
        }
    }
}
