use crate::syntax::expr::ExprId;
use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct PatternId {
    pub id: usize,
}

impl fmt::Display for PatternId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.id)
    }
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Binding(String),
    Tuple(Vec<PatternId>),
    Constructor(String, Vec<PatternId>),
    Guarded(PatternId, ExprId),
    Wildcard,
}
