use crate::environment::Environment;
use crate::interpreter::Interpreter;
use crate::value::Value;
use crate::value::ValueCore;
use siko_ir::expr::ExprId;
use siko_ir::function::NamedFunctionKind;
use siko_ir::types::ConcreteType;
use crate::util::get_opt_ordering_value;

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
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_int();
        let r = environment.get_arg_by_index(1).core.as_int();
        return Value::new(ValueCore::Int(l + r), ty);
    }
}

pub struct IntSub {}

impl ExternFunction for IntSub {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_int();
        let r = environment.get_arg_by_index(1).core.as_int();
        return Value::new(ValueCore::Int(l - r), ty);
    }
}

pub struct IntMul {}

impl ExternFunction for IntMul {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_int();
        let r = environment.get_arg_by_index(1).core.as_int();
        return Value::new(ValueCore::Int(l * r), ty);
    }
}

pub struct IntDiv {}

impl ExternFunction for IntDiv {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_int();
        let r = environment.get_arg_by_index(1).core.as_int();
        return Value::new(ValueCore::Int(l / r), ty);
    }
}

pub struct IntPartialEq {}

impl ExternFunction for IntPartialEq {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_int();
        let r = environment.get_arg_by_index(1).core.as_int();
        return Value::new(ValueCore::Bool(l == r), ty);
    }
}

pub struct IntPartialOrd {}

impl ExternFunction for IntPartialOrd {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value { 
         let l = environment.get_arg_by_index(0).core.as_int();
                        let r = environment.get_arg_by_index(1).core.as_int();
                        let ord = l.partial_cmp(&r);
                        return get_opt_ordering_value(ord);
    }
}

pub struct FloatAdd {}

impl ExternFunction for FloatAdd {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_float();
        let r = environment.get_arg_by_index(1).core.as_float();
        return Value::new(ValueCore::Float(l + r), ty);
    }
}

pub struct FloatSub {}

impl ExternFunction for FloatSub {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_float();
        let r = environment.get_arg_by_index(1).core.as_float();
        return Value::new(ValueCore::Float(l - r), ty);
    }
}

pub struct FloatMul {}

impl ExternFunction for FloatMul {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_float();
        let r = environment.get_arg_by_index(1).core.as_float();
        return Value::new(ValueCore::Float(l * r), ty);
    }
}

pub struct FloatDiv {}

impl ExternFunction for FloatDiv {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_float();
        let r = environment.get_arg_by_index(1).core.as_float();
        return Value::new(ValueCore::Float(l / r), ty);
    }
}

pub struct FloatPartialEq {}

impl ExternFunction for FloatPartialEq {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        let l = environment.get_arg_by_index(0).core.as_float();
        let r = environment.get_arg_by_index(1).core.as_float();
        return Value::new(ValueCore::Bool(l == r), ty);
    }
}

pub fn register_extern_functions(interpreter: &mut Interpreter) {
    // Int
    interpreter.add_extern_function("Data.Int", "opAdd", Box::new(IntAdd {}));
    interpreter.add_extern_function("Data.Int", "opSub", Box::new(IntSub {}));
    interpreter.add_extern_function("Data.Int", "opMul", Box::new(IntMul {}));
    interpreter.add_extern_function("Data.Int", "opDiv", Box::new(IntDiv {}));
    interpreter.add_extern_function("Data.Int", "opEq", Box::new(IntPartialEq {}));
    interpreter.add_extern_function("Data.Int", "partialCmp", Box::new(IntPartialOrd {}));
    // Float
    interpreter.add_extern_function("Data.Float", "opAdd", Box::new(FloatAdd {}));
    interpreter.add_extern_function("Data.Float", "opSub", Box::new(FloatSub {}));
    interpreter.add_extern_function("Data.Float", "opMul", Box::new(FloatMul {}));
    interpreter.add_extern_function("Data.Float", "opDiv", Box::new(FloatDiv {}));
    interpreter.add_extern_function("Data.Float", "opEq", Box::new(FloatPartialEq {}));
}
