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

pub struct Iter {}

impl ExternFunction for Iter {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: Type,
    ) -> Value {
        let list = environment.get_arg_by_index(0);
        return Value::new(ValueCore::Iterator(Box::new(list)), ty);
    }
}

pub struct ToList {}

impl ExternFunction for ToList {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: Type,
    ) -> Value {
        let iter = environment.get_arg_by_index(0);
        let iter = iter.core.as_iterator();
        let list: Vec<_> = iter.collect();
        return Value::new(ValueCore::List(list), ty);
    }
}

pub struct ListPartialEq {}

impl ExternFunction for ListPartialEq {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        _: Type,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_list();
        let r = environment.get_arg_by_index(1).core.as_list();
        for (a, b) in l.iter().zip(r.iter()) {
            let r = Interpreter::call_op_eq(a.clone(), b.clone());
            if !r.core.as_bool() {
                return r;
            }
        }
        return Interpreter::get_bool_value(true);
    }
}

pub fn register_extern_functions(interpreter: &mut Interpreter) {
    interpreter.add_extern_function(LIST_MODULE_NAME, "show", Box::new(Show {}));
    interpreter.add_extern_function(LIST_MODULE_NAME, "iter", Box::new(Iter {}));
    interpreter.add_extern_function(LIST_MODULE_NAME, "toList", Box::new(ToList {}));
    interpreter.add_extern_function(LIST_MODULE_NAME, "opEq", Box::new(ListPartialEq {}));
}
