use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::program::Program;

pub trait Collector {
    fn process(&mut self, program: &Program, expr: &Expr, id: ExprId);
}
