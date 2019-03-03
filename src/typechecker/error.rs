use crate::syntax::expr::ExprId;
use crate::syntax::function::FunctionId;

#[derive(Debug)]
pub enum TypecheckError {
    UntypedExternFunction(String, FunctionId),
    FunctionTypeDependencyLoop,
    IfBranchMismatch(ExprId, String, String),
    IfCondition(ExprId, String),
    TooManyArguments(ExprId, String, usize, usize),
    TypeMismatch(ExprId, String, String),
}
