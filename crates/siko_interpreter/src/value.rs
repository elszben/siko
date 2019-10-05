use crate::interpreter::eval_with_context;
use crate::interpreter::Interpreter;
use siko_ir::function::FunctionId;
use siko_ir::types::ConcreteType;
use siko_ir::types::SubstitutionContext;
use siko_ir::types::TypeDefId;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Callable {
    pub function_id: FunctionId,
    pub values: Vec<Value>,
    pub sub_context: SubstitutionContext,
}

#[derive(Debug, Clone)]
pub struct Value {
    pub core: ValueCore,
    pub ty: ConcreteType,
}

impl Value {
    pub fn new(core: ValueCore, ty: ConcreteType) -> Value {
        Value { core: core, ty: ty }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        let copy = self.clone();
        let other = other.clone();
        let func: Box<dyn FnOnce(&Interpreter) -> Value> =
            Box::new(|interpreter: &Interpreter| interpreter.call_op_eq(copy, other));
        let v = eval_with_context(func);
        v.core.as_bool()
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let copy = self.clone();
        let other = other.clone();
        let func: Box<dyn FnOnce(&Interpreter) -> Value> =
            Box::new(|interpreter: &Interpreter| interpreter.call_op_partial_cmp(copy, other));
        let v = eval_with_context(func);
        match v.core.as_option(0, 1) {
            Some(v) => Some(v.core.as_ordering(0, 1, 2)),
            None => None,
        }
    }
}

impl Eq for Value {}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        let copy = self.clone();
        let other = other.clone();
        let func: Box<dyn FnOnce(&Interpreter) -> Value> =
            Box::new(|interpreter: &Interpreter| interpreter.call_op_cmp(copy, other));
        let v = eval_with_context(func);
        v.core.as_ordering(0, 1, 2)
    }
}

#[derive(Debug, Clone)]
pub enum ValueCore {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Tuple(Vec<Value>),
    Callable(Callable),
    Variant(TypeDefId, usize, Vec<Value>),
    Record(TypeDefId, Vec<Value>),
    List(Vec<Value>),
    Map(BTreeMap<Value, Value>),
}

impl ValueCore {
    pub fn as_int(&self) -> i64 {
        match self {
            ValueCore::Int(i) => *i,
            _ => unreachable!(),
        }
    }

    pub fn as_float(&self) -> f64 {
        match self {
            ValueCore::Float(i) => *i,
            _ => unreachable!(),
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            ValueCore::String(i) => i.clone(),
            _ => unreachable!(),
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            ValueCore::Bool(b) => *b,
            _ => unreachable!(),
        }
    }

    pub fn as_simple_enum_variant(&self) -> (TypeDefId, usize) {
        match self {
            ValueCore::Variant(id, index, _) => (id.clone(), index.clone()),
            _ => unreachable!(),
        }
    }

    pub fn as_option(&self, some_index: usize, none_index: usize) -> Option<Value> {
        match self {
            ValueCore::Variant(_, index, items) if *index == some_index => Some(items[0].clone()),
            ValueCore::Variant(_, index, _) if *index == none_index => None,
            _ => unreachable!(),
        }
    }

    pub fn as_ordering(
        &self,
        less_index: usize,
        equal_index: usize,
        greater_index: usize,
    ) -> Ordering {
        match self {
            ValueCore::Variant(_, index, _) if *index == less_index => Ordering::Less,
            ValueCore::Variant(_, index, _) if *index == equal_index => Ordering::Equal,
            ValueCore::Variant(_, index, _) if *index == greater_index => Ordering::Greater,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for ValueCore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValueCore::Int(v) => write!(f, "{}", v),
            ValueCore::Float(v) => write!(f, "{}", v),
            ValueCore::Bool(v) => write!(f, "{}", v),
            ValueCore::String(v) => write!(f, "{}", v),
            ValueCore::Tuple(vs) => {
                let ss: Vec<_> = vs.iter().map(|v| format!("{}", v.core)).collect();
                write!(f, "({})", ss.join(", "))
            }
            ValueCore::Callable(_) => write!(f, "<closure>"),
            ValueCore::Variant(id, index, vs) => {
                let ss: Vec<_> = vs.iter().map(|v| format!("{}", v.core)).collect();
                write!(f, "V([{}/{}]{})", id, index, ss.join(", "))
            }
            ValueCore::Record(id, vs) => {
                let ss: Vec<_> = vs.iter().map(|v| format!("{}", v.core)).collect();
                write!(f, "R([{}]{})", id, ss.join(", "))
            }
            ValueCore::List(vs) => {
                let ss: Vec<_> = vs.iter().map(|v| format!("{}", v.core)).collect();
                write!(f, "[{}]", ss.join(", "))
            }
            ValueCore::Map(vs) => {
                let ss: Vec<_> = vs
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k.core, v.core))
                    .collect();
                write!(f, "{{{}}}", ss.join(", "))
            }
        }
    }
}
