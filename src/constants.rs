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

pub const MAIN_MODULE: &str = "Main";
pub const MAIN_FUNCTION: &str = "main";
pub const PRELUDE_NAME: &str = "Prelude";
