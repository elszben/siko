use crate::dfg::ValueId;
use siko_ir::expr::FunctionArgumentRef;
use siko_ir::function::FunctionId;
use siko_ir::pattern::PatternId;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub enum BuiltinCallable {
    Show,
    PartialEq,
    PartialOrd,
    Ord,
}

#[derive(Debug, Clone, Copy)]
pub enum CallableKind {
    FunctionId(FunctionId),
    Builtin(BuiltinCallable),
}

#[derive(Debug)]
pub struct Environment<'a> {
    callable_kind: CallableKind,
    args: Vec<ValueId>,
    variables: BTreeMap<PatternId, ValueId>,
    parent: Option<&'a Environment<'a>>,
}

impl<'a> Environment<'a> {
    pub fn new(callable_kind: CallableKind, args: Vec<ValueId>) -> Environment<'a> {
        Environment {
            callable_kind: callable_kind,
            args: args,
            variables: BTreeMap::new(),
            parent: None,
        }
    }

    pub fn add(&mut self, var: PatternId, value: ValueId) {
        self.variables.insert(var, value);
    }

    pub fn get_value(&self, var: &PatternId) -> ValueId {
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
            callable_kind: parent.callable_kind,
            args: parent.args.clone(),
            variables: BTreeMap::new(),
            parent: Some(parent),
        }
    }

    pub fn get_arg(&self, arg_ref: &FunctionArgumentRef) -> ValueId {
        if let CallableKind::FunctionId(id) = self.callable_kind {
            if id == arg_ref.id {
                return self.args[arg_ref.index].clone();
            }
        }
        if let Some(parent) = self.parent {
            return parent.get_arg(arg_ref);
        } else {
            unreachable!()
        }
    }

    pub fn get_arg_by_index(&self, index: usize) -> ValueId {
        return self.args[index].clone();
    }
}
