use crate::value::Value;
use siko_ir::expr::FunctionArgumentRef;
use siko_ir::function::FunctionId;
use siko_ir::pattern::PatternId;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Environment<'a> {
    function_id: FunctionId,
    args: Vec<Value>,
    variables: BTreeMap<PatternId, Value>,
    parent: Option<&'a Environment<'a>>,
    captured_arg_count: usize,
}

impl<'a> Environment<'a> {
    pub fn new(
        function_id: FunctionId,
        args: Vec<Value>,
        captured_arg_count: usize,
    ) -> Environment<'a> {
        Environment {
            function_id: function_id,
            args: args,
            variables: BTreeMap::new(),
            parent: None,
            captured_arg_count: captured_arg_count,
        }
    }

    pub fn add(&mut self, var: PatternId, value: Value) {
        self.variables.insert(var, value);
    }

    pub fn get_value(&self, var: &PatternId) -> Value {
        if let Some(value) = self.variables.get(var) {
            return value.clone();
        } else {
            if let Some(parent) = self.parent {
                parent.get_value(var)
            } else {
                panic!("Value {} not found", var);
            }
        }
    }

    pub fn block_child(parent: &'a Environment<'a>) -> Environment<'a> {
        Environment {
            function_id: parent.function_id,
            args: parent.args.clone(),
            variables: BTreeMap::new(),
            parent: Some(parent),
            captured_arg_count: parent.captured_arg_count,
        }
    }

    pub fn get_arg(&self, arg_ref: &FunctionArgumentRef) -> Value {
        if self.function_id == arg_ref.id {
            let index = if arg_ref.captured {
                arg_ref.index
            } else {
                arg_ref.index + self.captured_arg_count
            };
            return self.args[index].clone();
        } else {
            if let Some(parent) = self.parent {
                return parent.get_arg(arg_ref);
            } else {
                unreachable!()
            }
        }
    }

    pub fn get_arg_by_index(&self, index: usize) -> Value {
        return self.args[index].clone();
    }
}
