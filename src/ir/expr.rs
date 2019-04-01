use crate::ir::function::FunctionId;
use crate::ir::types::TypeDefId;
use crate::location_info::item::LocationId;
use crate::syntax::expr::ExprId as AstExprId;
use crate::util::format_list;
use std::fmt;

#[derive(Debug, Clone)]
pub struct FunctionArgumentRef {
    pub id: FunctionId,
    pub index: usize,
}

impl FunctionArgumentRef {
    pub fn new(id: FunctionId, index: usize) -> FunctionArgumentRef {
        FunctionArgumentRef {
            id: id,
            index: index,
        }
    }
}

impl fmt::Display for FunctionArgumentRef {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ArgRef({}.{})", self.id, self.index)
    }
}

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
struct FieldAccessInfo {
    record_id: TypeDefId,
    index: usize,
    name: String,
}

impl fmt::Display for FieldAccessInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "FieldAccessInfo{}:{}({})",
            self.record_id, self.index, self.name
        )
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    StaticFunctionCall(FunctionId, Vec<ExprId>),
    LambdaFunction(FunctionId, Vec<ExprId>),
    DynamicFunctionCall(ExprId, Vec<ExprId>),
    If(ExprId, ExprId, ExprId),
    Tuple(Vec<ExprId>),
    IntegerLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),
    Do(Vec<ExprId>),
    Bind(String, ExprId),
    ArgRef(FunctionArgumentRef),
    ExprValue(ExprId),
    LambdaCapturedArgRef(FunctionArgumentRef),
    FieldAccess(Vec<FieldAccessInfo>, ExprId),
    TupleFieldAccess(usize, ExprId),
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::StaticFunctionCall(id, args) => {
                write!(f, "StaticFunctionCall({}, {})", id, format_list(args))
            }
            Expr::LambdaFunction(id, args) => {
                write!(f, "LambdaFunction({}, {})", id, format_list(args))
            }
            Expr::DynamicFunctionCall(id_expr, args) => {
                write!(f, "DynamicFunctionCall({}, {})", id_expr, format_list(args))
            }
            Expr::If(cond, true_branch, false_branch) => {
                write!(f, "If({}, {}, {})", cond, true_branch, false_branch)
            }
            Expr::Tuple(items) => write!(f, "Tuple({})", format_list(items)),
            Expr::IntegerLiteral(v) => write!(f, "Integer({})", v),
            Expr::FloatLiteral(v) => write!(f, "Float({})", v),
            Expr::BoolLiteral(v) => write!(f, "Bool({})", v),
            Expr::StringLiteral(v) => write!(f, "String({})", v),
            Expr::Do(items) => write!(f, "Do({})", format_list(items)),
            Expr::Bind(t, expr) => write!(f, "Bind({}, {})", t, expr),
            Expr::ArgRef(v) => write!(f, "{}", v),
            Expr::ExprValue(id) => write!(f, "ExprValue({})", id),
            Expr::LambdaCapturedArgRef(v) => write!(f, "LambdaCaptured({})", v),
            Expr::FieldAccess(accesses, expr) => {
                write!(f, "FieldAccess({}, {})", format_list(accesses), expr)
            }
            Expr::TupleFieldAccess(index, expr) => {
                write!(f, "TupleFieldAccess({}, {})", index, expr)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExprInfo {
    pub expr: Expr,
    pub ast_expr_id: AstExprId,
    pub location_id: LocationId,
}

impl ExprInfo {
    pub fn new(expr: Expr, ast_expr_id: AstExprId, location_id: LocationId) -> ExprInfo {
        ExprInfo {
            expr: expr,
            ast_expr_id: ast_expr_id,
            location_id: location_id,
        }
    }
}
