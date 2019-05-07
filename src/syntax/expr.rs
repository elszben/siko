use crate::constants::BuiltinOperator;
use crate::location_info::item::LocationId;
use crate::util::format_list;
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

#[derive(Debug, Clone)]
pub enum Expr {
    Lambda(Vec<(String, LocationId)>, ExprId),
    FunctionCall(ExprId, Vec<ExprId>),
    Builtin(BuiltinOperator),
    If(ExprId, ExprId, ExprId),
    Tuple(Vec<ExprId>),
    Path(String),
    IntegerLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),
    Do(Vec<ExprId>),
    Bind(String, ExprId),
    FieldAccess(String, ExprId),
    TupleFieldAccess(usize, ExprId),
    Formatter(String, Vec<ExprId>),
    Case(ExprId),
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Lambda(args, body) => {
                let args: Vec<_> = args.iter().map(|arg| &arg.0).collect();
                write!(f, "Lambda({}, {})", format_list(&args[..]), body)
            }
            Expr::FunctionCall(expr, args) => {
                write!(f, "FunctionCall({}, {})", expr, format_list(args))
            }
            Expr::Builtin(op) => write!(f, "Op({:?})", op),

            Expr::If(cond, true_branch, false_branch) => {
                write!(f, "If({}, {}, {})", cond, true_branch, false_branch)
            }
            Expr::Tuple(items) => write!(f, "Tuple({})", format_list(items)),
            Expr::Path(path) => write!(f, "Path({})", path),
            Expr::IntegerLiteral(v) => write!(f, "Integer({})", v),
            Expr::FloatLiteral(v) => write!(f, "Float({})", v),
            Expr::BoolLiteral(v) => write!(f, "Bool({})", v),
            Expr::StringLiteral(v) => write!(f, "String({})", v),
            Expr::Do(items) => write!(f, "Do({})", format_list(items)),
            Expr::Bind(t, expr) => write!(f, "Bind({}, {})", t, expr),
            Expr::FieldAccess(name, expr) => write!(f, "FieldAccess({}, {})", name, expr),
            Expr::TupleFieldAccess(index, expr) => {
                write!(f, "TupleFieldAccess({}, {})", index, expr)
            }
            Expr::Formatter(fmt, items) => write!(f, "Formatter({}, {})", fmt, format_list(items)),
            Expr::Case(body) => write!(f, "Case({})", body),
        }
    }
}
