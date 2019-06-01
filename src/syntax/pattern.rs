use crate::location_info::item::LocationId;
use crate::syntax::expr::ExprId;
use crate::syntax::types::TypeSignatureId;
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
pub struct RecordFieldPattern {
    pub name: String,
    pub value: PatternId,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Binding(String),
    Tuple(Vec<PatternId>),
    Constructor(String, Vec<PatternId>),
    Guarded(PatternId, ExprId),
    Wildcard,
    IntegerLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BoolLiteral(bool),
    Typed(PatternId, TypeSignatureId),
    Record(String, Vec<RecordFieldPattern>),
}
