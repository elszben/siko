use crate::environment::Environment;
use crate::extern_function::ExternFunction;
use crate::interpreter::Interpreter;
use crate::util::get_opt_ordering_value;
use crate::util::get_ordering_value;
use crate::value::Value;
use crate::value::ValueCore;
use siko_ir::expr::ExprId;
use siko_ir::function::NamedFunctionKind;
use siko_ir::types::Type;

pub struct StringAdd {}

impl ExternFunction for StringAdd {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: Type,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_string();
        let r = environment.get_arg_by_index(1).core.as_string();
        return Value::new(ValueCore::String(l + &r), ty);
    }
}

pub struct StringPartialEq {}

impl ExternFunction for StringPartialEq {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: Type,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_string();
        let r = environment.get_arg_by_index(1).core.as_string();
        return Value::new(ValueCore::Bool(l == r), ty);
    }
}

pub struct StringPartialOrd {}

impl ExternFunction for StringPartialOrd {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        _: Type,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_string();
        let r = environment.get_arg_by_index(1).core.as_string();
        let ord = l.partial_cmp(&r);
        return get_opt_ordering_value(ord);
    }
}

pub struct StringOrd {}

impl ExternFunction for StringOrd {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        _: Type,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_string();
        let r = environment.get_arg_by_index(1).core.as_string();
        let ord = l.cmp(&r);
        return get_ordering_value(ord);
    }
}

pub struct StringShow {}

impl ExternFunction for StringShow {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: Type,
    ) -> Value {
        let value = environment.get_arg_by_index(0).core.as_string();
        return Value::new(ValueCore::String(value.to_string()), ty);
    }
}

pub fn register_extern_functions(interpreter: &mut Interpreter) {
    interpreter.add_extern_function("Data.String", "opAdd", Box::new(StringAdd {}));
    interpreter.add_extern_function("Data.String", "opEq", Box::new(StringPartialEq {}));
    interpreter.add_extern_function("Data.String", "partialCmp", Box::new(StringPartialOrd {}));
    interpreter.add_extern_function("Data.String", "cmp", Box::new(StringOrd {}));
    interpreter.add_extern_function("Data.String", "show", Box::new(StringShow {}));
}
