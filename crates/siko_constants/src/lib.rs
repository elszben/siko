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
}

impl BuiltinOperator {
    pub fn get_function_name(&self) -> String {
        match self {
            BuiltinOperator::Add => format!("Std.Ops.opAdd"),
            BuiltinOperator::Sub => format!("Std.Ops.opSub"),
            BuiltinOperator::Mul => format!("Std.Ops.opMul"),
            BuiltinOperator::Div => format!("Std.Ops.opDiv"),
            BuiltinOperator::Equals => format!("Std.Ops.opEq"),
            BuiltinOperator::NotEquals => format!("Std.Ops.opNotEq"),
            BuiltinOperator::LessThan => format!("Std.Ops.opLessThan"),
            BuiltinOperator::LessOrEqualThan => format!("Std.Ops.opLessEqual"),
            BuiltinOperator::GreaterThan => format!("Std.Ops.opGreaterThan"),
            BuiltinOperator::GreaterOrEqualThan => format!("Std.Ops.opGreaterEqual"),
            BuiltinOperator::And => format!("Std.Ops.opAnd"),
            BuiltinOperator::Or => format!("Std.Ops.opOr"),
            BuiltinOperator::Not => format!("Std.Ops.opNot"),
            _ => panic!("Op {:?} has no func name", self),
        }
    }
}

pub const MAIN_MODULE: &str = "Main";
pub const MAIN_FUNCTION: &str = "main";
pub const OPTION_NAME: &str = "Option";
pub const ORDERING_NAME: &str = "Ordering";
pub const INT_NAME: &str = "Int";
pub const FLOAT_NAME: &str = "Float";
pub const BOOL_NAME: &str = "Bool";
pub const STRING_NAME: &str = "String";
pub const FULL_LIST_NAME: &str = "Prelude.List";
pub const LIST_NAME: &str = "List";
