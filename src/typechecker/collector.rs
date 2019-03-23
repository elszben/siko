use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::program::Program;

pub trait Collector {
    fn process(&mut self, program: &Program, expr: &Expr, id: ExprId);
}
