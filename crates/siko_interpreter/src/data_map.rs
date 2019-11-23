use crate::environment::Environment;
use crate::extern_function::ExternFunction;
use crate::interpreter::Interpreter;
use crate::util::create_none;
use crate::util::create_some;
use crate::value::Value;
use crate::value::ValueCore;
use siko_ir::expr::ExprId;
use siko_ir::function::NamedFunctionKind;
use siko_ir::types::Type;
use std::collections::BTreeMap;

pub struct Empty {}

impl ExternFunction for Empty {
    fn call(
        &self,
        _: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: Type,
    ) -> Value {
        return Value::new(ValueCore::Map(BTreeMap::new()), ty);
    }
}

pub struct Insert {}

impl ExternFunction for Insert {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: Type,
    ) -> Value {
        let mut first_arg = environment.get_arg_by_index(0);
        let mut map_type_args = first_arg.ty.get_type_args();
        let mut map = first_arg.core.as_map();
        let key = environment.get_arg_by_index(1);
        let value = environment.get_arg_by_index(2);
        let res = map.insert(key, value);
        let v = match res {
            Some(v) => create_some(v),
            None => create_none(map_type_args.remove(1)),
        };
        first_arg.core = ValueCore::Map(map);
        let tuple = Value::new(ValueCore::Tuple(vec![first_arg, v]), ty);
        return tuple;
    }
}

pub struct Remove {}

impl ExternFunction for Remove {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        ty: Type,
    ) -> Value {
        let mut first_arg = environment.get_arg_by_index(0);
        let mut map_type_args = first_arg.ty.get_type_args();
        let mut map = first_arg.core.as_map();
        let key = environment.get_arg_by_index(1);
        let res = map.remove(&key);
        let v = match res {
            Some(v) => create_some(v),
            None => create_none(map_type_args.remove(1)),
        };
        first_arg.core = ValueCore::Map(map);
        let tuple = Value::new(ValueCore::Tuple(vec![first_arg, v]), ty);
        return tuple;
    }
}

pub struct Get {}

impl ExternFunction for Get {
    fn call(
        &self,
        environment: &mut Environment,
        _: Option<ExprId>,
        _: &NamedFunctionKind,
        _: Type,
    ) -> Value {
        let first_arg = environment.get_arg_by_index(0);
        let mut map_type_args = first_arg.ty.get_type_args();
        let map = first_arg.core.as_map();
        let key = environment.get_arg_by_index(1);
        let res = map.get(&key);
        let v = match res {
            Some(v) => create_some(v.clone()),
            None => create_none(map_type_args.remove(1)),
        };
        return v;
    }
}

pub fn register_extern_functions(interpreter: &mut Interpreter) {
    interpreter.add_extern_function("Data.Map", "empty", Box::new(Empty {}));
    interpreter.add_extern_function("Data.Map", "insert", Box::new(Insert {}));
    interpreter.add_extern_function("Data.Map", "remove", Box::new(Remove {}));
    interpreter.add_extern_function("Data.Map", "get", Box::new(Get {}));
}
