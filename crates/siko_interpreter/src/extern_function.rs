use crate::environment::Environment;
use crate::value::Value;
use crate::value::ValueCore;
use siko_ir::expr::ExprId;
use siko_ir::function::NamedFunctionKind;
use siko_ir::types::ConcreteType;

fn get_instance_name_from_kind(kind: &NamedFunctionKind) -> &str {
    if let NamedFunctionKind::InstanceMember(Some(s)) = kind {
        s.as_ref()
    } else {
        unreachable!()
    }
}

pub trait ExternFunction {
    fn call(
        &self,
        environment: &mut Environment,
        current_expr: Option<ExprId>,
        kind: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value;
}

pub struct IntAdd {}

impl ExternFunction for IntAdd {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        kind: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_int();
        let r = environment.get_arg_by_index(1).core.as_int();
        return Value::new(ValueCore::Int(l + r), ty);
    }
}
