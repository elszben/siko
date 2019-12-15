use crate::environment::Environment;
use crate::extern_function::ExternFunction;
use crate::interpreter::Interpreter;
use crate::value::Value;
use crate::value::ValueCore;
use siko_constants::LIST_MODULE_NAME;
use siko_ir::expr::ExprId;
use siko_ir::function::NamedFunctionKind;
use siko_ir::types::Type;

pub struct Show {}

impl ExternFunction for Show {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: Type,
    ) -> Value {
        let list = environment.get_arg_by_index(0).core.as_list();
        let mut subs = Vec::new();
        for item in list {
            let s = Interpreter::call_show(item);
            subs.push(s);
        }
        return Value::new(ValueCore::String(format!("[{}]", subs.join(", "))), ty);
    }
}

pub fn register_extern_functions(interpreter: &mut Interpreter) {
    interpreter.add_extern_function(LIST_MODULE_NAME, "show", Box::new(Show {}));
}
