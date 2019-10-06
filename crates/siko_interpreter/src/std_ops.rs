use crate::environment::Environment;
use crate::extern_function::ExternFunction;
use crate::interpreter::Interpreter;
use crate::value::Value;
use crate::value::ValueCore;
use siko_ir::expr::ExprId;
use siko_ir::function::NamedFunctionKind;
use siko_ir::types::ConcreteType;

pub struct And {}

impl ExternFunction for And {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_bool();
        let r = environment.get_arg_by_index(1).core.as_bool();
        return Value::new(ValueCore::Bool(l && r), ty);
    }
}

pub struct Or {}

impl ExternFunction for Or {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_bool();
        if l {
            return Value::new(ValueCore::Bool(l), ty);
        } else {
            let r = environment.get_arg_by_index(1).core.as_bool();
            return Value::new(ValueCore::Bool(r), ty);
        }
    }
}

pub fn register_extern_functions(interpreter: &mut Interpreter) {
    interpreter.add_extern_function("Std.Ops", "opAnd", Box::new(And {}));
    interpreter.add_extern_function("Std.Ops", "opOr", Box::new(Or {}));
}
