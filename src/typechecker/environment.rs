use super::type_variable::TypeVariable;
use super::types::Type;
use std::collections::BTreeMap;

pub struct Environment<'a> {
    variables: BTreeMap<String, (TypeVariable, Type)>,
    parent: Option<&'a Environment<'a>>,
}

impl<'a> Environment<'a> {
    pub fn new() -> Environment<'a> {
        Environment {
            variables: BTreeMap::new(),
            parent: None,
        }
    }

    pub fn add(&mut self, var: String, value: TypeVariable, ty: Type) {
        self.variables.insert(var, (value, ty));
    }

    pub fn get_value(&self, var: &str) -> (TypeVariable, Type) {
        if let Some(value) = self.variables.get(var) {
            return value.clone();
        } else {
            if let Some(parent) = self.parent {
                parent.get_value(var)
            } else {
                panic!("TypeVariable {} not found", var);
            }
        }
    }

    fn child(parent: &'a Environment<'a>) -> Environment<'a> {
        Environment {
            variables: BTreeMap::new(),
            parent: Some(parent),
        }
    }
}
