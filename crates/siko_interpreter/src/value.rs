use siko_constants::BOOL_NAME;
use siko_constants::FLOAT_NAME;
use siko_constants::INT_NAME;
use siko_constants::LIST_NAME;
use siko_constants::STRING_NAME;
use siko_ir::function::FunctionId;
use siko_ir::program::Program;
use siko_ir::types::ConcreteType;
use siko_ir::types::SubstitutionContext;
use siko_ir::types::TypeDefId;
use siko_ir::types::TypeId;
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
    pub ty: TypeId,
}

impl Value {
    pub fn new(core: ValueCore, ty: TypeId) -> Value {
        Value { core: core, ty: ty }
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
}

impl ValueCore {
    pub fn as_int(&self) -> i64 {
        match self {
            ValueCore::Int(i) => *i,
            _ => unreachable!(),
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            ValueCore::Bool(b) => *b,
            _ => unreachable!(),
        }
    }

    pub fn debug(&self, program: &Program, inner: bool) -> String {
        let mut parens_needed = false;
        let v = match self {
            ValueCore::Int(v) => format!("{}", v),
            ValueCore::Float(v) => format!("{}", v),
            ValueCore::Bool(v) => format!("{}", v),
            ValueCore::String(v) => format!("{}", v),
            ValueCore::Tuple(vs) => {
                let ss: Vec<_> = vs.iter().map(|v| v.core.debug(program, true)).collect();
                format!("({})", ss.join(", "))
            }
            ValueCore::Callable(_) => format!("<closure>"),
            ValueCore::Variant(id, index, vs) => {
                parens_needed = !vs.is_empty();
                let ss: Vec<_> = vs.iter().map(|v| v.core.debug(program, true)).collect();
                let adt = program.typedefs.get(id).get_adt();
                let variant = &adt.variants[*index];
                format!("{} {}", variant.name, ss.join(" "))
            }
            ValueCore::Record(id, vs) => {
                let ss: Vec<_> = vs.iter().map(|v| v.core.debug(program, true)).collect();
                let record = program.typedefs.get(id).get_record();
                format!("{} {}", record.name, ss.join(" "))
            }
            ValueCore::List(vs) => {
                let ss: Vec<_> = vs.iter().map(|v| v.core.debug(program, true)).collect();
                format!("[{}]", ss.join(", "))
            }
        };
        if inner && parens_needed {
            format!("({})", v)
        } else {
            v
        }
    }

    pub fn to_type(&self, program: &Program) -> ConcreteType {
        match self {
            ValueCore::Int(v) => ConcreteType::Named(
                INT_NAME.to_string(),
                program.builtin_types.int_id.unwrap(),
                vec![],
            ),
            ValueCore::Float(v) => ConcreteType::Named(
                FLOAT_NAME.to_string(),
                program.builtin_types.float_id.unwrap(),
                vec![],
            ),
            ValueCore::Bool(v) => ConcreteType::Named(
                BOOL_NAME.to_string(),
                program.builtin_types.bool_id.unwrap(),
                vec![],
            ),
            ValueCore::String(v) => ConcreteType::Named(
                STRING_NAME.to_string(),
                program.builtin_types.string_id.unwrap(),
                vec![],
            ),
            ValueCore::Tuple(vs) => {
                let items: Vec<_> = vs.iter().map(|v| v.core.to_type(program)).collect();
                ConcreteType::Tuple(items)
            }
            ValueCore::Callable(_) => unimplemented!(),
            ValueCore::Variant(_, _, _) => unimplemented!(),
            ValueCore::Record(id, vs) => {
                let items: Vec<_> = vs.iter().map(|v| v.core.to_type(program)).collect();
                let record = program.typedefs.get(id).get_record();
                ConcreteType::Named(record.name.clone(), *id, items)
            }
            ValueCore::List(vs) => {
                let item_type = vs[0].core.to_type(program);
                ConcreteType::Named(
                    LIST_NAME.to_string(),
                    program.builtin_types.list_id.unwrap(),
                    vec![item_type],
                )
            }
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
        }
    }
}
