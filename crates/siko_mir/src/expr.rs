use crate::function::FunctionId;
use crate::pattern::PatternId;
use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ExprId {
    pub id: usize,
}

impl fmt::Display for ExprId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.id)
    }
}

impl From<usize> for ExprId {
    fn from(id: usize) -> ExprId {
        ExprId { id: id }
    }
}

pub enum Expr {
    ArgRef(usize),
    Bind(PatternId, ExprId),
    Do(Vec<ExprId>),
    DynamicFunctionCall(ExprId, Vec<ExprId>),
    ExprValue(ExprId, PatternId),
    FloatLiteral(f64),
    Formatter(String, Vec<ExprId>),
    IntegerLiteral(i64),
    List(Vec<ExprId>),
    StaticFunctionCall(FunctionId, Vec<ExprId>),
    StringLiteral(String),
    Tuple(Vec<ExprId>),
}
