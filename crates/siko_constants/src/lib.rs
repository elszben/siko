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
pub const BOOL_MODULE_NAME: &str = "Bool";
pub const BOOL_TYPE_NAME: &str = "Bool";
pub const INT_MODULE_NAME: &str = "Int";
pub const INT_TYPE_NAME: &str = "Int";
pub const FLOAT_MODULE_NAME: &str = "Float";
pub const FLOAT_TYPE_NAME: &str = "Float";
pub const OPTION_MODULE_NAME: &str = "Option";
pub const OPTION_TYPE_NAME: &str = "Option";
pub const RESULT_MODULE_NAME: &str = "Result";
pub const RESULT_TYPE_NAME: &str = "Result";
pub const MAP_MODULE_NAME: &str = "Map";
pub const MAP_TYPE_NAME: &str = "Map";
pub const ORDERING_MODULE_NAME: &str = "Ordering";
pub const ORDERING_TYPE_NAME: &str = "Ordering";
pub const STRING_MODULE_NAME: &str = "String";
pub const STRING_TYPE_NAME: &str = "String";
pub const LIST_MODULE_NAME: &str = "List";
pub const LIST_TYPE_NAME: &str = "List";

pub fn get_qualified_list_type_name() -> String {
    format!("{}.{}", LIST_MODULE_NAME, LIST_TYPE_NAME)
}
