use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;

pub trait Visitor {
    fn visit_expr(&mut self, expr_id: ExprId, expr: &Expr);
    fn visit_pattern(&mut self, pattern_id: PatternId, pattern: &Pattern);
}

pub fn walk_expr(expr_id: &ExprId, program: &Program, visitor: &mut Visitor) {
    let expr = &program.exprs.get(expr_id).item;
    match expr {
        Expr::StaticFunctionCall(_, args) => {
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
        Expr::Bind(bind_pattern, rhs) => {
            walk_expr(rhs, program, visitor);
            walk_pattern(bind_pattern, program, visitor);
        }
        Expr::ArgRef(_) => {}
        Expr::ExprValue(_, _) => {}
        Expr::FieldAccess(_, lhs) => {
            walk_expr(lhs, program, visitor);
        }
        Expr::TupleFieldAccess(_, lhs) => {
            walk_expr(lhs, program, visitor);
        }
        Expr::Formatter(_, items) => {
            for item in items {
                walk_expr(item, program, visitor);
            }
        }
        Expr::CaseOf(body, cases) => {
            walk_expr(body, program, visitor);
            for case in cases {
                walk_expr(&case.body, program, visitor);
                walk_pattern(&case.pattern_id, program, visitor);
            }
        }
        Expr::RecordInitialization(_, items) => {
            for item in items {
                walk_expr(&item.expr_id, program, visitor);
            }
        }
        Expr::RecordUpdate(record_expr_id, updates) => {
            walk_expr(record_expr_id, program, visitor);
            for update in updates {
                for item in &update.items {
                    walk_expr(&item.expr_id, program, visitor);
                }
            }
        }
        Expr::ClassFunctionCall(_, args) => {
            for arg in args {
                walk_expr(arg, program, visitor);
            }
        }
    }
    visitor.visit_expr(*expr_id, expr);
}

fn walk_pattern(pattern_id: &PatternId, program: &Program, visitor: &mut Visitor) {
    let pattern = &program.patterns.get(pattern_id).item;
    match pattern {
        Pattern::Binding(_) => {}
        Pattern::Tuple(items) => {
            for item in items {
                walk_pattern(item, program, visitor);
            }
        }
        Pattern::Record(_, items) => {
            for item in items {
                walk_pattern(item, program, visitor);
            }
        }
        Pattern::Variant(_, _, items) => {
            for item in items {
                walk_pattern(item, program, visitor);
            }
        }
        Pattern::Guarded(id, expr_id) => {
            walk_pattern(id, program, visitor);
            walk_expr(expr_id, program, visitor);
        }
        Pattern::Wildcard => {}
        Pattern::IntegerLiteral(_) => {}
        Pattern::FloatLiteral(_) => {}
        Pattern::StringLiteral(_) => {}
        Pattern::BoolLiteral(_) => {}
    }
    visitor.visit_pattern(*pattern_id, pattern);
}
