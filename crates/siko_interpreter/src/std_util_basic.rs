use crate::environment::Environment;
use crate::extern_function::ExternFunction;
use crate::interpreter::Interpreter;
use crate::value::Value;
use crate::value::ValueCore;
use siko_ir::expr::ExprId;
use siko_ir::function::NamedFunctionKind;
use siko_ir::types_old::ConcreteType;

pub struct Print {}

impl ExternFunction for Print {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let v = environment.get_arg_by_index(0).core.as_string();
        print!("{}", v);
        return Value::new(ValueCore::Tuple(vec![]), ty);
    }
}

pub struct PrintLn {}

impl ExternFunction for PrintLn {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let v = environment.get_arg_by_index(0).core.as_string();
        println!("{}", v);
        return Value::new(ValueCore::Tuple(vec![]), ty);
    }
}

pub fn register_extern_functions(interpreter: &mut Interpreter) {
    interpreter.add_extern_function("Std.Util.Basic", "print", Box::new(Print {}));
    interpreter.add_extern_function("Std.Util.Basic", "println", Box::new(PrintLn {}));
}
