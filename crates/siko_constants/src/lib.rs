#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum BuiltinOperator {
    Add,
    Sub,
    Mul,
    Div,
    PipeForward,
    And,
    Or,
    Equals,
    NotEquals,
    LessThan,
    GreaterThan,
    LessOrEqualThan,
    GreaterOrEqualThan,
    Not,
    Minus,
    Bind,
    Arrow,
    Composition,
}

impl BuiltinOperator {
    pub fn get_function_name(&self) -> String {
        match self {
            BuiltinOperator::Add => format!("opAdd"),
            BuiltinOperator::Sub => format!("opSub"),
            BuiltinOperator::Mul => format!("opMul"),
            BuiltinOperator::Div => format!("opDiv"),
            BuiltinOperator::Equals => format!("opEq"),
            BuiltinOperator::NotEquals => format!("opNotEq"),
            _ => panic!("Op {:?} has no func name", self),
        }
    }
}

pub const MAIN_MODULE: &str = "Main";
pub const MAIN_FUNCTION: &str = "main";
pub const PRELUDE_NAME: &str = "Prelude";
pub const INT_NAME: &str = "Int";
pub const FLOAT_NAME: &str = "Float";
pub const BOOL_NAME: &str = "Bool";
pub const STRING_NAME: &str = "String";
pub const FULL_LIST_NAME: &str = "Prelude.List";
pub const LIST_NAME: &str = "List";
