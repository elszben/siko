use crate::ir::program::Program;
use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;

pub trait Collector {
    fn process(&mut self, program: &Program, expr: &Expr, id: ExprId);
}
