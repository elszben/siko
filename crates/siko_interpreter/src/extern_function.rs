use crate::environment::Environment;
use crate::value::Value;
use siko_ir::expr::ExprId;
use siko_ir::function::NamedFunctionKind;
use siko_ir::types::Type;

pub trait ExternFunction {
    fn call(
        &self,
        environment: &mut Environment,
        current_expr: Option<ExprId>,
        kind: &NamedFunctionKind,
        ty: Type,
    ) -> Value;
}
