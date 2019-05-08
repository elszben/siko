use crate::ir::function::FunctionId;
use crate::ir::program::Program;
use crate::ir::types::TypeDefId;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Callable {
    pub function_id: FunctionId,
    pub values: Vec<Value>,
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Tuple(Vec<Value>),
    Callable(Callable),
    Variant(TypeDefId, usize, Vec<Value>),
    Record(TypeDefId, Vec<Value>),
}

impl Value {
    pub fn as_int(&self) -> i64 {
        match self {
            Value::Int(i) => *i,
            _ => unreachable!(),
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            _ => unreachable!(),
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            _ => unreachable!(),
        }
    }

    pub fn debug(&self, program: &Program) -> String {
        match self {
            Value::Int(v) => format!("{}", v),
            Value::Float(v) => format!("{}", v),
            Value::Bool(v) => format!("{}", v),
            Value::String(v) => format!("{}", v),
            Value::Tuple(vs) => {
                let ss: Vec<_> = vs.iter().map(|v| v.debug(program)).collect();
                format!("({})", ss.join(", "))
            }
            Value::Callable(_) => format!("<closure>"),
            Value::Variant(id, index, vs) => {
                let ss: Vec<_> = vs.iter().map(|v| v.debug(program)).collect();
                let adt = program.get_adt(id);
                let variant = &adt.variants[*index];
                format!("{} {}", variant.name, ss.join(" "))
            }
            Value::Record(id, vs) => {
                let ss: Vec<_> = vs.iter().map(|v| v.debug(program)).collect();
                let record = program.get_record(id);
                format!("{} {}", record.name, ss.join(" "))
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{}", v),
            Value::Float(v) => write!(f, "{}", v),
            Value::Bool(v) => write!(f, "{}", v),
            Value::String(v) => write!(f, "{}", v),
            Value::Tuple(vs) => {
                let ss: Vec<_> = vs.iter().map(|v| format!("{}", v)).collect();
                write!(f, "({})", ss.join(", "))
            }
            Value::Callable(_) => write!(f, "<closure>"),
            Value::Variant(id, index, vs) => {
                let ss: Vec<_> = vs.iter().map(|v| format!("{}", v)).collect();
                write!(f, "V([{}/{}]{})", id, index, ss.join(", "))
            }
            Value::Record(id, vs) => {
                let ss: Vec<_> = vs.iter().map(|v| format!("{}", v)).collect();
                write!(f, "R([{}]{})", id, ss.join(", "))
            }
        }
    }
}
