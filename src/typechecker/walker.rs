use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::program::Program;

pub trait Visitor {
    fn visit(&mut self, expr_id: ExprId, expr: &Expr);
}

pub fn walk_expr(expr_id: &ExprId, program: &Program, visitor: &mut Visitor) {
    let expr = program.get_expr(expr_id);
    match expr {
        Expr::StaticFunctionCall(_, args) => {
            for arg in args {
                walk_expr(arg, program, visitor);
            }
        }
        Expr::LambdaFunction(_, args) => {
            for arg in args {
                walk_expr(arg, program, visitor);
            }
        }
        Expr::DynamicFunctionCall(func_expr, args) => {
            walk_expr(func_expr, program, visitor);
            for arg in args {
                walk_expr(arg, program, visitor);
            }
        }
        Expr::If(cond, true_branch, false_branch) => {
            walk_expr(cond, program, visitor);
            walk_expr(true_branch, program, visitor);
            walk_expr(false_branch, program, visitor);
        }
        Expr::Tuple(items) => {
            for item in items {
                walk_expr(item, program, visitor);
            }
        }
        Expr::IntegerLiteral(_) => {}
        Expr::FloatLiteral(_) => {}
        Expr::BoolLiteral(_) => {}
        Expr::StringLiteral(_) => {}
        Expr::Do(items) => {
            for item in items {
                walk_expr(item, program, visitor);
            }
        }
        Expr::Bind(_, rhs) => {
            walk_expr(rhs, program, visitor);
        }
        Expr::ArgRef(_) => {}
        Expr::ExprValue(_) => {}
        Expr::LambdaCapturedArgRef(_) => {}
        Expr::FieldAccess(_, lhs) => {
            walk_expr(lhs, program, visitor);
        }
        Expr::TupleFieldAccess(_, lhs) => {
            walk_expr(lhs, program, visitor);
        }
    }
    visitor.visit(*expr_id, expr);
}
