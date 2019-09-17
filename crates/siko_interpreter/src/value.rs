use siko_constants::BOOL_NAME;
use siko_constants::FLOAT_NAME;
use siko_constants::INT_NAME;
use siko_constants::LIST_NAME;
use siko_constants::STRING_NAME;
use siko_ir::function::FunctionId;
use siko_ir::program::Program;
use siko_ir::types::Type;
use siko_ir::types::TypeDefId;
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
    List(Vec<Value>),
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

    pub fn debug(&self, program: &Program, inner: bool) -> String {
        let mut parens_needed = false;
        let v = match self {
            Value::Int(v) => format!("{}", v),
            Value::Float(v) => format!("{}", v),
            Value::Bool(v) => format!("{}", v),
            Value::String(v) => format!("{}", v),
            Value::Tuple(vs) => {
                let ss: Vec<_> = vs.iter().map(|v| v.debug(program, true)).collect();
                format!("({})", ss.join(", "))
            }
            Value::Callable(_) => format!("<closure>"),
            Value::Variant(id, index, vs) => {
                parens_needed = !vs.is_empty();
                let ss: Vec<_> = vs.iter().map(|v| v.debug(program, true)).collect();
                let adt = program.typedefs.get(id).get_adt();
                let variant = &adt.variants[*index];
                format!("{} {}", variant.name, ss.join(" "))
            }
            Value::Record(id, vs) => {
                let ss: Vec<_> = vs.iter().map(|v| v.debug(program, true)).collect();
                let record = program.typedefs.get(id).get_record();
                format!("{} {}", record.name, ss.join(" "))
            }
            Value::List(vs) => {
                let ss: Vec<_> = vs.iter().map(|v| v.debug(program, true)).collect();
                format!("[{}]", ss.join(", "))
            }
        };
        if inner && parens_needed {
            format!("({})", v)
        } else {
            v
        }
    }

    pub fn to_type(&self, program: &Program) -> Type {
        match self {
            Value::Int(v) => Type::Named(
                INT_NAME.to_string(),
                program.builtin_types.int_id.unwrap(),
                vec![],
            ),
            Value::Float(v) => Type::Named(
                FLOAT_NAME.to_string(),
                program.builtin_types.float_id.unwrap(),
                vec![],
            ),
            Value::Bool(v) => Type::Named(
                BOOL_NAME.to_string(),
                program.builtin_types.bool_id.unwrap(),
                vec![],
            ),
            Value::String(v) => Type::Named(
                STRING_NAME.to_string(),
                program.builtin_types.string_id.unwrap(),
                vec![],
            ),
            Value::Tuple(vs) => {
                let items: Vec<_> = vs.iter().map(|v| v.to_type(program)).collect();
                Type::Tuple(items)
            }
            Value::Callable(_) => unimplemented!(),
            Value::Variant(_, _, _) => unimplemented!(),
            Value::Record(id, vs) => {
                let items: Vec<_> = vs.iter().map(|v| v.to_type(program)).collect();
                let record = program.typedefs.get(id).get_record();
                Type::Named(record.name.clone(), *id, items)
            }
            Value::List(vs) => {
                let item_type = vs[0].to_type(program);
                Type::Named(
                    LIST_NAME.to_string(),
                    program.builtin_types.list_id.unwrap(),
                    vec![item_type],
                )
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
            Value::List(vs) => {
                let ss: Vec<_> = vs.iter().map(|v| format!("{}", v)).collect();
                write!(f, "[{}]", ss.join(", "))
            }
        }
    }
}
