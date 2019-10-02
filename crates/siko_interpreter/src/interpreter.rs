use crate::environment::Environment;
use crate::value::Callable;
use crate::value::Value;
use crate::value::ValueCore;
use siko_constants::MAIN_FUNCTION;
use siko_constants::MAIN_MODULE;
use siko_constants::PRELUDE_NAME;
use siko_ir::class::ClassMemberId;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::function::FunctionInfo;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_ir::types::ConcreteType;
use siko_ir::types::SubstitutionContext;
use siko_ir::types::Type;
use siko_ir::types::TypeId;
use siko_location_info::error_context::ErrorContext;

pub struct Interpreter<'a> {
    error_context: ErrorContext<'a>,
}

impl<'a> Interpreter<'a> {
    pub fn new(error_context: ErrorContext<'a>) -> Interpreter<'a> {
        Interpreter {
            error_context: error_context,
        }
    }

    fn get_func_arg_types(
        &self,
        program: &Program,
        ty_id: &TypeId,
        arg_count: usize,
    ) -> (Vec<TypeId>, TypeId) {
        let (func_arg_types, return_type) = match program.types.get(ty_id).expect("type not found")
        {
            Type::Function(func_type) => {
                let mut arg_types = Vec::new();
                if arg_count == 0 {
                    (arg_types, *ty_id)
                } else {
                    let return_type =
                        func_type.get_arg_and_return_types(program, &mut arg_types, arg_count);
                    (arg_types, return_type)
                }
            }
            _ => (vec![], *ty_id),
        };
        (func_arg_types, return_type)
    }

    fn get_subtitution_context(
        &self,
        program: &Program,
        func_arg_types: &[TypeId],
        args: &[Value],
        return_type: &ConcreteType,
        func_return_type: &TypeId,
    ) -> SubstitutionContext {
        let mut sub_context = SubstitutionContext::new();
        for (arg_type, func_arg_type) in args.iter().zip(func_arg_types.iter()) {
            program.match_generic_types(&arg_type.ty, func_arg_type, &mut sub_context);
        }
        program.match_generic_types(&return_type, func_return_type, &mut sub_context);
        sub_context
    }

    fn call(
        &mut self,
        callable_value: Value,
        args: Vec<Value>,
        program: &Program,
        expr_id: Option<ExprId>,
    ) -> Value {
        match callable_value.core {
            ValueCore::Callable(mut callable) => {
                let mut callable_func_ty = callable_value.ty;
                callable.values.extend(args);
                loop {
                    let func_info = program.functions.get(&callable.function_id);
                    let needed_arg_count =
                        func_info.arg_locations.len() + func_info.implicit_arg_count;
                    if needed_arg_count > callable.values.len() {
                        return Value::new(ValueCore::Callable(callable), callable_func_ty);
                    } else {
                        let rest = callable.values.split_off(needed_arg_count);
                        let mut call_args = Vec::new();
                        std::mem::swap(&mut call_args, &mut callable.values);
                        let arg_count = call_args.len();
                        let mut environment = Environment::new(
                            callable.function_id,
                            call_args,
                            func_info.implicit_arg_count,
                        );
                        callable_func_ty = callable_func_ty.get_func_type(arg_count);
                        let result = self.execute(
                            program,
                            callable.function_id,
                            &mut environment,
                            expr_id,
                            &callable.sub_context,
                            callable_func_ty.clone(),
                        );
                        if !rest.is_empty() {
                            if let ValueCore::Callable(new_callable) = result.core {
                                callable = new_callable;
                                callable.values.extend(rest);
                            } else {
                                unreachable!()
                            }
                        } else {
                            return result;
                        }
                    }
                }
            }
            _ => panic!("Cannot call {:?}", callable_value),
        }
    }

    fn match_pattern(
        &mut self,
        pattern_id: &PatternId,
        value: &Value,
        program: &Program,
        environment: &mut Environment,
        sub_context: &SubstitutionContext,
    ) -> bool {
        let pattern = &program.patterns.get(pattern_id).item;
        match pattern {
            Pattern::Binding(_) => {
                environment.add(*pattern_id, value.clone());
                return true;
            }
            Pattern::Tuple(ids) => match &value.core {
                ValueCore::Tuple(vs) => {
                    for (index, id) in ids.iter().enumerate() {
                        let v = &vs[index];
                        if !self.match_pattern(id, v, program, environment, sub_context) {
                            return false;
                        }
                    }
                    return true;
                }
                _ => {
                    return false;
                }
            },
            Pattern::Record(p_type_id, p_ids) => match &value.core {
                ValueCore::Record(type_id, vs) => {
                    if type_id == p_type_id {
                        for (index, p_id) in p_ids.iter().enumerate() {
                            let v = &vs[index];
                            if !self.match_pattern(p_id, v, program, environment, sub_context) {
                                return false;
                            }
                        }
                        return true;
                    }
                    return false;
                }
                _ => {
                    return false;
                }
            },
            Pattern::Variant(p_type_id, p_index, p_ids) => match &value.core {
                ValueCore::Variant(type_id, index, vs) => {
                    if type_id == p_type_id && index == p_index {
                        for (index, p_id) in p_ids.iter().enumerate() {
                            let v = &vs[index];
                            if !self.match_pattern(p_id, v, program, environment, sub_context) {
                                return false;
                            }
                        }
                        return true;
                    }
                    return false;
                }
                _ => {
                    return false;
                }
            },
            Pattern::Guarded(id, guard_expr_id) => {
                if self.match_pattern(id, value, program, environment, sub_context) {
                    let guard_value =
                        self.eval_expr(program, *guard_expr_id, environment, sub_context);
                    return guard_value.core.as_bool();
                } else {
                    return false;
                }
            }
            Pattern::Typed(id, _) => {
                self.match_pattern(id, value, program, environment, sub_context)
            }
            Pattern::Wildcard => {
                return true;
            }
            Pattern::IntegerLiteral(p_v) => {
                let r = match &value.core {
                    ValueCore::Int(v) => p_v == v,
                    _ => false,
                };
                return r;
            }
            Pattern::FloatLiteral(p_v) => {
                let r = match &value.core {
                    ValueCore::Float(v) => p_v == v,
                    _ => false,
                };
                return r;
            }
            Pattern::StringLiteral(p_v) => {
                let r = match &value.core {
                    ValueCore::String(v) => p_v == v,
                    _ => false,
                };
                return r;
            }
            Pattern::BoolLiteral(p_v) => {
                let r = match &value.core {
                    ValueCore::Bool(v) => p_v == v,
                    _ => false,
                };
                return r;
            }
        }
    }

    fn call_show(&mut self, program: &Program, arg: Value) -> String {
        let string_ty = program.string_concrete_type();
        let class_id = program.class_names.get("Show").expect("Show not found");
        let class = program.classes.get(class_id);
        let class_member_id = class.members.get("show").expect("show not found");
        let v =
            self.call_class_member(program, class_member_id, vec![arg], None, string_ty.clone());
        if let ValueCore::String(s) = v.core {
            return s;
        } else {
            unreachable!();
        }
    }

    fn call_class_member(
        &mut self,
        program: &Program,
        class_member_id: &ClassMemberId,
        arg_values: Vec<Value>,
        expr_id: Option<ExprId>,
        expr_ty: ConcreteType,
    ) -> Value {
        let member = program.class_members.get(class_member_id);
        let (class_member_type_id, class_arg_ty_id) = program
            .class_member_types
            .get(class_member_id)
            .expect("untyped class member");
        let (func_arg_types, return_type) =
            self.get_func_arg_types(program, class_member_type_id, arg_values.len());
        let callee_sub_context = self.get_subtitution_context(
            program,
            &func_arg_types[..],
            &arg_values[..],
            &expr_ty,
            &return_type,
        );
        let instance_selector_ty = program.to_concrete_type(class_arg_ty_id, &callee_sub_context);
        let concrete_function_type =
            program.to_concrete_type(class_member_type_id, &callee_sub_context);
        //println!("instance selector {} {}", instance_selector_ty, member.name);
        let resolver = program.type_instance_resolver.borrow();
        if let Some(instances) = resolver.instance_map.get(&member.class_id) {
            if let Some(instance_id) = instances.get(&instance_selector_ty) {
                let instance = program.instances.get(instance_id);
                let instance_member = instance.members.get(&member.name).unwrap();
                let callable = Value::new(
                    ValueCore::Callable(Callable {
                        function_id: instance_member.function_id,
                        values: vec![],
                        sub_context: callee_sub_context,
                    }),
                    concrete_function_type,
                );
                return self.call(callable, arg_values, program, expr_id);
            } else {
                for (a, b) in instances {
                    println!("{} {}", a, b);
                }
                panic!("Did not find {}", instance_selector_ty);
            }
        } else {
            unreachable!()
        }
    }

    fn eval_expr(
        &mut self,
        program: &Program,
        expr_id: ExprId,
        environment: &mut Environment,
        sub_context: &SubstitutionContext,
    ) -> Value {
        let expr = &program.exprs.get(&expr_id).item;
        //println!("Eval {} {}", expr_id, expr);
        let expr_ty_id = program
            .expr_types
            .get(&expr_id)
            .expect("Untyped expr")
            .clone();
        let expr_ty = program.to_concrete_type(&expr_ty_id, sub_context);
        match expr {
            Expr::IntegerLiteral(v) => Value::new(ValueCore::Int(*v), expr_ty),
            Expr::StringLiteral(v) => Value::new(ValueCore::String(v.clone()), expr_ty),
            Expr::FloatLiteral(v) => Value::new(ValueCore::Float(v.clone()), expr_ty),
            Expr::BoolLiteral(v) => Value::new(ValueCore::Bool(v.clone()), expr_ty),
            Expr::ArgRef(arg_ref) => {
                return environment.get_arg(arg_ref);
            }
            Expr::StaticFunctionCall(function_id, args) => {
                let func_ty = program
                    .function_types
                    .get(function_id)
                    .expect("untyped func");
                let (func_arg_types, return_type) =
                    self.get_func_arg_types(program, func_ty, args.len());
                let arg_values: Vec<_> = args
                    .iter()
                    .map(|arg| self.eval_expr(program, *arg, environment, sub_context))
                    .collect();
                let callee_sub_context = self.get_subtitution_context(
                    program,
                    &func_arg_types[..],
                    &arg_values[..],
                    &expr_ty,
                    &return_type,
                );
                let concrete_function_type = program.to_concrete_type(func_ty, &callee_sub_context);
                let callable = Value::new(
                    ValueCore::Callable(Callable {
                        function_id: *function_id,
                        values: vec![],
                        sub_context: callee_sub_context,
                    }),
                    concrete_function_type,
                );
                return self.call(callable, arg_values, program, Some(expr_id));
            }
            Expr::DynamicFunctionCall(function_expr_id, args) => {
                let function_expr_id =
                    self.eval_expr(program, *function_expr_id, environment, sub_context);
                let arg_values: Vec<_> = args
                    .iter()
                    .map(|arg| self.eval_expr(program, *arg, environment, sub_context))
                    .collect();
                return self.call(function_expr_id, arg_values, program, Some(expr_id));
            }
            Expr::Do(exprs) => {
                let mut environment = Environment::block_child(environment);
                let mut result = Value::new(ValueCore::Tuple(vec![]), expr_ty);
                assert!(!exprs.is_empty());
                for expr in exprs {
                    result = self.eval_expr(program, *expr, &mut environment, sub_context);
                }
                return result;
            }
            Expr::Bind(pattern_id, expr_id) => {
                let value = self.eval_expr(program, *expr_id, environment, sub_context);
                let r = self.match_pattern(pattern_id, &value, program, environment, sub_context);
                assert!(r);
                return Value::new(ValueCore::Tuple(vec![]), expr_ty);
            }
            Expr::ExprValue(_, pattern_id) => {
                return environment.get_value(pattern_id);
            }
            Expr::If(cond, true_branch, false_branch) => {
                let cond_value = self.eval_expr(program, *cond, environment, sub_context);
                if cond_value.core.as_bool() {
                    return self.eval_expr(program, *true_branch, environment, sub_context);
                } else {
                    return self.eval_expr(program, *false_branch, environment, sub_context);
                }
            }
            Expr::Tuple(exprs) => {
                let values: Vec<_> = exprs
                    .iter()
                    .map(|e| self.eval_expr(program, *e, environment, sub_context))
                    .collect();
                return Value::new(ValueCore::Tuple(values), expr_ty);
            }
            Expr::List(exprs) => {
                let values: Vec<_> = exprs
                    .iter()
                    .map(|e| self.eval_expr(program, *e, environment, sub_context))
                    .collect();
                return Value::new(ValueCore::List(values), expr_ty);
            }
            Expr::TupleFieldAccess(index, tuple) => {
                let tuple_value = self.eval_expr(program, *tuple, environment, sub_context);
                if let ValueCore::Tuple(t) = &tuple_value.core {
                    return t[*index].clone();
                } else {
                    unreachable!()
                }
            }
            Expr::Formatter(fmt, args) => {
                let subs: Vec<_> = fmt.split("{}").collect();
                let values: Vec<_> = args
                    .iter()
                    .map(|e| self.eval_expr(program, *e, environment, sub_context))
                    .collect();
                let mut result = String::new();
                for (index, sub) in subs.iter().enumerate() {
                    result += sub;
                    if values.len() > index {
                        let value_as_string = self.call_show(program, values[index].clone());
                        result += &value_as_string;
                    }
                }
                return Value::new(ValueCore::String(result), expr_ty);
            }
            Expr::FieldAccess(infos, record_expr) => {
                let record = self.eval_expr(program, *record_expr, environment, sub_context);
                let (id, values) = if let ValueCore::Record(id, values) = &record.core {
                    (id, values)
                } else {
                    unreachable!()
                };
                for info in infos {
                    if info.record_id != *id {
                        continue;
                    }
                    return values[info.index].clone();
                }
                unreachable!()
            }
            Expr::CaseOf(body, cases) => {
                let case_value = self.eval_expr(program, *body, environment, sub_context);
                for case in cases {
                    let mut case_env = Environment::block_child(environment);
                    if self.match_pattern(
                        &case.pattern_id,
                        &case_value,
                        program,
                        &mut case_env,
                        sub_context,
                    ) {
                        let val = self.eval_expr(program, case.body, &mut case_env, sub_context);
                        return val;
                    }
                }
                unreachable!()
            }
            Expr::RecordInitialization(type_id, items) => {
                let mut values: Vec<_> = Vec::with_capacity(items.len());
                for _ in 0..items.len() {
                    values.push(Value::new(ValueCore::Bool(false), expr_ty.clone())); // dummy value
                }
                for item in items {
                    let value = self.eval_expr(program, item.expr_id, environment, sub_context);
                    values[item.index] = value;
                }
                return Value::new(ValueCore::Record(*type_id, values), expr_ty);
            }
            Expr::RecordUpdate(record_expr_id, updates) => {
                let value = self.eval_expr(program, *record_expr_id, environment, sub_context);
                if let ValueCore::Record(id, mut values) = value.core {
                    for update in updates {
                        if id == update.record_id {
                            for item in &update.items {
                                let value =
                                    self.eval_expr(program, item.expr_id, environment, sub_context);
                                values[item.index] = value;
                            }
                            return Value::new(ValueCore::Record(id, values), expr_ty);
                        }
                    }
                }
                unreachable!()
            }
            Expr::ClassFunctionCall(class_member_id, args) => {
                let arg_values: Vec<_> = args
                    .iter()
                    .map(|e| self.eval_expr(program, *e, environment, sub_context))
                    .collect();
                return self.call_class_member(
                    program,
                    class_member_id,
                    arg_values,
                    Some(expr_id),
                    expr_ty,
                );
            }
        }
    }

    fn call_extern(
        &mut self,
        module: &str,
        name: &str,
        environment: &mut Environment,
        program: &Program,
        current_expr: Option<ExprId>,
        instance: Option<String>,
        ty: ConcreteType,
    ) -> Value {
        match (module, name) {
            (PRELUDE_NAME, "opAdd") => match instance {
                Some(instance_name) => match instance_name.as_ref() {
                    "IntAdd" => {
                        let l = environment.get_arg_by_index(0).core.as_int();
                        let r = environment.get_arg_by_index(1).core.as_int();
                        return Value::new(ValueCore::Int(l + r), ty);
                    }
                    "FloatAdd" => {
                        let l = environment.get_arg_by_index(0).core.as_float();
                        let r = environment.get_arg_by_index(1).core.as_float();
                        return Value::new(ValueCore::Float(l + r), ty);
                    }
                    "StringAdd" => {
                        let l = environment.get_arg_by_index(0).core.as_string();
                        let r = environment.get_arg_by_index(1).core.as_string();
                        return Value::new(ValueCore::String(l + &r), ty);
                    }
                    _ => {
                        panic!("Unimplemented add function {}/{}", module, instance_name);
                    }
                },
                None => unreachable!(),
            },
            (PRELUDE_NAME, "opSub") => match instance {
                Some(instance_name) => match instance_name.as_ref() {
                    "IntSub" => {
                        let l = environment.get_arg_by_index(0).core.as_int();
                        let r = environment.get_arg_by_index(1).core.as_int();
                        return Value::new(ValueCore::Int(l - r), ty);
                    }
                    "FloatSub" => {
                        let l = environment.get_arg_by_index(0).core.as_float();
                        let r = environment.get_arg_by_index(1).core.as_float();
                        return Value::new(ValueCore::Float(l - r), ty);
                    }
                    _ => {
                        panic!("Unimplemented sub function {}/{}", module, instance_name);
                    }
                },
                None => unreachable!(),
            },
            (PRELUDE_NAME, "opMul") => match instance {
                Some(instance_name) => match instance_name.as_ref() {
                    "IntMul" => {
                        let l = environment.get_arg_by_index(0).core.as_int();
                        let r = environment.get_arg_by_index(1).core.as_int();
                        return Value::new(ValueCore::Int(l * r), ty);
                    }
                    "FloatMul" => {
                        let l = environment.get_arg_by_index(0).core.as_float();
                        let r = environment.get_arg_by_index(1).core.as_float();
                        return Value::new(ValueCore::Float(l * r), ty);
                    }
                    _ => {
                        panic!("Unimplemented sub function {}/{}", module, instance_name);
                    }
                },
                None => unreachable!(),
            },
            (PRELUDE_NAME, "opDiv") => match instance {
                Some(instance_name) => match instance_name.as_ref() {
                    "IntDiv" => {
                        let l = environment.get_arg_by_index(0).core.as_int();
                        let r = environment.get_arg_by_index(1).core.as_int();
                        return Value::new(ValueCore::Int(l / r), ty);
                    }
                    "FloatDiv" => {
                        let l = environment.get_arg_by_index(0).core.as_float();
                        let r = environment.get_arg_by_index(1).core.as_float();
                        return Value::new(ValueCore::Float(l / r), ty);
                    }
                    _ => {
                        panic!("Unimplemented sub function {}/{}", module, instance_name);
                    }
                },
                None => unreachable!(),
            },
            (PRELUDE_NAME, "opEq") => match instance {
                Some(instance_name) => match instance_name.as_ref() {
                    "BoolEq" => {
                        let l = environment.get_arg_by_index(0).core.as_bool();
                        let r = environment.get_arg_by_index(1).core.as_bool();
                        return Value::new(ValueCore::Bool(l == r), ty);
                    }
                    "IntEq" => {
                        let l = environment.get_arg_by_index(0).core.as_int();
                        let r = environment.get_arg_by_index(1).core.as_int();
                        return Value::new(ValueCore::Bool(l == r), ty);
                    }
                    "FloatEq" => {
                        let l = environment.get_arg_by_index(0).core.as_float();
                        let r = environment.get_arg_by_index(1).core.as_float();
                        return Value::new(ValueCore::Bool(l == r), ty);
                    }
                    "StringEq" => {
                        let l = environment.get_arg_by_index(0).core.as_string();
                        let r = environment.get_arg_by_index(1).core.as_string();
                        return Value::new(ValueCore::Bool(l == r), ty);
                    }
                    _ => {
                        panic!("Unimplemented eq function {}/{}", module, instance_name);
                    }
                },
                None => unreachable!(),
            },
            (PRELUDE_NAME, "opNotEq") => match instance {
                Some(instance_name) => match instance_name.as_ref() {
                    "BoolEq" => {
                        let l = environment.get_arg_by_index(0).core.as_bool();
                        let r = environment.get_arg_by_index(1).core.as_bool();
                        return Value::new(ValueCore::Bool(l != r), ty);
                    }
                    _ => {
                        panic!("Unimplemented notEq function {}/{}", module, instance_name);
                    }
                },
                None => unreachable!(),
            },
            (PRELUDE_NAME, "opLessThan") => match instance {
                Some(instance_name) => match instance_name.as_ref() {
                    "IntLessThan" => {
                        let l = environment.get_arg_by_index(0).core.as_int();
                        let r = environment.get_arg_by_index(1).core.as_int();
                        return Value::new(ValueCore::Bool(l < r), ty);
                    }
                    _ => {
                        panic!(
                            "Unimplemented less than function {}/{}",
                            module, instance_name
                        );
                    }
                },
                None => unreachable!(),
            },
            (PRELUDE_NAME, "op_lessthan") => {
                let l = environment.get_arg_by_index(0).core.as_int();
                let r = environment.get_arg_by_index(1).core.as_int();
                return Value::new(ValueCore::Bool(l < r), ty);
            }
            ("Std.Util", "assert") => {
                let v = environment.get_arg_by_index(0).core.as_bool();
                if !v {
                    let current_expr = current_expr.expect("No current expr");
                    let location_id = program.exprs.get(&current_expr).location_id;
                    self.error_context
                        .report_error(format!("Assertion failed"), location_id);
                    panic!("Abort not implemented");
                }
                return Value::new(ValueCore::Tuple(vec![]), ty);
            }
            (PRELUDE_NAME, "print") => {
                let v = environment.get_arg_by_index(0).core.debug(program, false);
                print!("{}", v);
                return Value::new(ValueCore::Tuple(vec![]), ty);
            }
            (PRELUDE_NAME, "println") => {
                let v = environment.get_arg_by_index(0).core.debug(program, false);
                println!("{}", v);
                return Value::new(ValueCore::Tuple(vec![]), ty);
            }
            (PRELUDE_NAME, "show") => match instance {
                Some(instance_name) => match instance_name.as_ref() {
                    "ListShow" => {
                        let list = environment.get_arg_by_index(0);
                        if let ValueCore::List(items) = list.core {
                            let mut subs = Vec::new();
                            for item in items {
                                let s = self.call_show(program, item);
                                subs.push(s);
                            }
                            return Value::new(
                                ValueCore::String(format!("({})", subs.join(", "))),
                                ty,
                            );
                        } else {
                            unreachable!()
                        }
                    }
                    "IntShow" => {
                        let value = environment.get_arg_by_index(0);
                        return Value::new(ValueCore::String(value.core.debug(program, false)), ty);
                    }
                    "FloatShow" => {
                        let value = environment.get_arg_by_index(0);
                        return Value::new(ValueCore::String(value.core.debug(program, false)), ty);
                    }
                    _ => {
                        panic!("Unimplemented show function {}/{}", module, instance_name);
                    }
                },
                None => unreachable!(),
            },
            _ => {
                panic!("Unimplemented extern function {}/{}", module, name);
            }
        }
    }

    fn execute(
        &mut self,
        program: &Program,
        id: FunctionId,
        environment: &mut Environment,
        current_expr: Option<ExprId>,
        sub_context: &SubstitutionContext,
        expr_ty: ConcreteType,
    ) -> Value {
        let function = program.functions.get(&id);
        match &function.info {
            FunctionInfo::NamedFunction(info) => match info.body {
                Some(body) => {
                    return self.eval_expr(program, body, environment, sub_context);
                }
                None => {
                    return self.call_extern(
                        &info.module,
                        &info.name,
                        environment,
                        program,
                        current_expr,
                        info.instance.clone(),
                        expr_ty,
                    );
                }
            },
            FunctionInfo::Lambda(info) => {
                return self.eval_expr(program, info.body, environment, sub_context);
            }
            FunctionInfo::VariantConstructor(info) => {
                let adt = program.typedefs.get(&info.type_id).get_adt();
                let variant = &adt.variants[info.index];
                let mut values = Vec::new();
                for index in 0..variant.items.len() {
                    let v = environment.get_arg_by_index(index);
                    values.push(v);
                }
                return Value::new(
                    ValueCore::Variant(info.type_id, info.index, values),
                    expr_ty,
                );
            }
            FunctionInfo::RecordConstructor(info) => {
                let record = program.typedefs.get(&info.type_id).get_record();
                let mut values = Vec::new();
                for index in 0..record.fields.len() {
                    let v = environment.get_arg_by_index(index);
                    values.push(v);
                }
                return Value::new(ValueCore::Record(info.type_id, values), expr_ty);
            }
        }
    }

    pub fn run(&mut self, program: &Program) -> Value {
        for (id, function) in &program.functions.items {
            match &function.info {
                FunctionInfo::NamedFunction(info) => {
                    if info.module == MAIN_MODULE && info.name == MAIN_FUNCTION {
                        let mut environment = Environment::new(*id, vec![], 0);
                        let sub_context = SubstitutionContext::new();
                        return self.execute(
                            program,
                            *id,
                            &mut environment,
                            None,
                            &sub_context,
                            ConcreteType::Tuple(vec![]),
                        );
                    }
                }
                _ => {}
            }
        }

        panic!(
            "Cannot find function {} in module {}",
            MAIN_FUNCTION, MAIN_MODULE
        );
    }
}
