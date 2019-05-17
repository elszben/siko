use crate::ir::expr::ExprId;
use crate::ir::types::TypeDefId;
use crate::location_info::item::LocationId;
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
    Record(TypeDefId, Vec<PatternId>),
    Variant(TypeDefId, usize, Vec<PatternId>),
    Guarded(PatternId, ExprId),
    Wildcard,
    IntegerLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),
}

#[derive(Debug, Clone)]
pub struct PatternInfo {
    pub pattern: Pattern,
    pub location_id: LocationId,
}