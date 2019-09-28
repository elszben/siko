use crate::environment::Environment;
use crate::value::Callable;
use crate::value::Value;
use crate::value::ValueCore;
use siko_constants::MAIN_FUNCTION;
use siko_constants::MAIN_MODULE;
use siko_constants::PRELUDE_NAME;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::function::FunctionInfo;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
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

    fn get_func_arg_types(&self, program: &Program, ty_id: &TypeId) -> Vec<TypeId> {
        let func_arg_types = match program.types.get(ty_id).expect("type not found") {
            Type::Function(func_type) => {
                let mut arg_types = Vec::new();
                func_type.get_arg_types(program, &mut arg_types);
                arg_types
            }
            _ => vec![],
        };
        func_arg_types
    }

    fn get_callee_subtitution_context(
        &self,
        program: &Program,
        func_arg_types: &[TypeId],
        args: &[ExprId],
        caller_context: &SubstitutionContext,
    ) -> SubstitutionContext {
        let mut sub_context = SubstitutionContext::new();
        let mut arg_types = Vec::new();
        for arg in args {
            let arg_ty = program.expr_types.get(arg).expect("untyped expr");
            arg_types.push(arg_ty);
        }
        for (arg_type, func_arg_type) in arg_types.iter().zip(func_arg_types.iter()) {
            program.match_generic_types(arg_type, func_arg_type, caller_context, &mut sub_context);
        }
        sub_context
    }

    fn call(
        &mut self,
        callable: Value,
        args: Vec<Value>,
        program: &Program,
        expr_id: ExprId,
        ty_id: TypeId,
    ) -> Value {
        match callable.core {
            ValueCore::Callable(mut callable) => {
                callable.values.extend(args);
                loop {
                    let func_info = program.functions.get(&callable.function_id);
                    let needed_arg_count =
                        func_info.arg_locations.len() + func_info.implicit_arg_count;
                    if needed_arg_count > callable.values.len() {
                        return Value::new(ValueCore::Callable(callable), ty_id);
                    } else {
                        let rest = callable.values.split_off(needed_arg_count);
                        let mut call_args = Vec::new();
                        std::mem::swap(&mut call_args, &mut callable.values);
                        let mut environment = Environment::new(
                            callable.function_id,
                            call_args,
                            func_info.implicit_arg_count,
                        );
                        let result = self.execute(
                            program,
                            callable.function_id,
                            &mut environment,
                            Some(expr_id),
                            &callable.sub_context,
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
            _ => panic!("Cannot call {:?}", callable),
        }
    }

    fn match_pattern(
        &mut self,
        pattern_id: &PatternId,
        value: &Value,
        program: &Program,
        environment: &mut Environment,
        caller_context: &SubstitutionContext,
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
                        if !self.match_pattern(id, v, program, environment, caller_context) {
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
                            if !self.match_pattern(p_id, v, program, environment, caller_context) {
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
                            if !self.match_pattern(p_id, v, program, environment, caller_context) {
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
                if self.match_pattern(id, value, program, environment, caller_context) {
                    let guard_value =
                        self.eval_expr(program, *guard_expr_id, environment, caller_context);
                    return guard_value.core.as_bool();
                } else {
                    return false;
                }
            }
            Pattern::Typed(id, _) => {
                self.match_pattern(id, value, program, environment, caller_context)
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

    fn eval_expr(
        &mut self,
        program: &Program,
        expr_id: ExprId,
        environment: &mut Environment,
        caller_context: &SubstitutionContext,
    ) -> Value {
        let expr = &program.exprs.get(&expr_id).item;
        //println!("Eval {} {}", expr_id, expr);
        let expr_ty_id = program
            .expr_types
            .get(&expr_id)
            .expect("Untyped expr")
            .clone();
        match expr {
            Expr::IntegerLiteral(v) => Value::new(ValueCore::Int(*v), expr_ty_id),
            Expr::StringLiteral(v) => Value::new(ValueCore::String(v.clone()), expr_ty_id),
            Expr::FloatLiteral(v) => Value::new(ValueCore::Float(v.clone()), expr_ty_id),
            Expr::BoolLiteral(v) => Value::new(ValueCore::Bool(v.clone()), expr_ty_id),
            Expr::ArgRef(arg_ref) => {
                return environment.get_arg(arg_ref);
            }
            Expr::StaticFunctionCall(function_id, args) => {
                let func_ty = program
                    .function_types
                    .get(function_id)
                    .expect("untyped func");
                let func_arg_types = self.get_func_arg_types(program, func_ty);
                let callee_sub_context = self.get_callee_subtitution_context(
                    program,
                    &func_arg_types[..],
                    args,
                    caller_context,
                );
                let callable = Value::new(
                    ValueCore::Callable(Callable {
                        function_id: *function_id,
                        values: vec![],
                        sub_context: callee_sub_context,
                    }),
                    expr_ty_id,
                );
                let arg_values: Vec<_> = args
                    .iter()
                    .map(|arg| self.eval_expr(program, *arg, environment, caller_context))
                    .collect();
                return self.call(callable, arg_values, program, expr_id, expr_ty_id);
            }
            Expr::DynamicFunctionCall(function_expr_id, args) => {
                let function_expr_id =
                    self.eval_expr(program, *function_expr_id, environment, caller_context);
                let arg_values: Vec<_> = args
                    .iter()
                    .map(|arg| self.eval_expr(program, *arg, environment, caller_context))
                    .collect();
                return self.call(function_expr_id, arg_values, program, expr_id, expr_ty_id);
            }
            Expr::Do(exprs) => {
                let mut environment = Environment::block_child(environment);
                let mut result = Value::new(ValueCore::Tuple(vec![]), expr_ty_id);
                assert!(!exprs.is_empty());
                for expr in exprs {
                    result = self.eval_expr(program, *expr, &mut environment, caller_context);
                }
                return result;
            }
            Expr::Bind(pattern_id, expr_id) => {
                let value = self.eval_expr(program, *expr_id, environment, caller_context);
                let r =
                    self.match_pattern(pattern_id, &value, program, environment, caller_context);
                assert!(r);
                return Value::new(ValueCore::Tuple(vec![]), expr_ty_id);
            }
            Expr::ExprValue(_, pattern_id) => {
                return environment.get_value(pattern_id);
            }
            Expr::If(cond, true_branch, false_branch) => {
                let cond_value = self.eval_expr(program, *cond, environment, caller_context);
                if cond_value.core.as_bool() {
                    return self.eval_expr(program, *true_branch, environment, caller_context);
                } else {
                    return self.eval_expr(program, *false_branch, environment, caller_context);
                }
            }
            Expr::Tuple(exprs) => {
                let values: Vec<_> = exprs
                    .iter()
                    .map(|e| self.eval_expr(program, *e, environment, caller_context))
                    .collect();
                return Value::new(ValueCore::Tuple(values), expr_ty_id);
            }
            Expr::List(exprs) => {
                let values: Vec<_> = exprs
                    .iter()
                    .map(|e| self.eval_expr(program, *e, environment, caller_context))
                    .collect();
                return Value::new(ValueCore::List(values), expr_ty_id);
            }
            Expr::TupleFieldAccess(index, tuple) => {
                let tuple_value = self.eval_expr(program, *tuple, environment, caller_context);
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
                    .map(|e| self.eval_expr(program, *e, environment, caller_context))
                    .collect();
                let mut result = String::new();
                for (index, sub) in subs.iter().enumerate() {
                    result += sub;
                    if values.len() > index {
                        result += &values[index].core.debug(program, false);
                    }
                }
                return Value::new(ValueCore::String(result), expr_ty_id);
            }
            Expr::FieldAccess(infos, record_expr) => {
                let record = self.eval_expr(program, *record_expr, environment, caller_context);
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
                let case_value = self.eval_expr(program, *body, environment, caller_context);
                for case in cases {
                    let mut case_env = Environment::block_child(environment);
                    if self.match_pattern(
                        &case.pattern_id,
                        &case_value,
                        program,
                        &mut case_env,
                        caller_context,
                    ) {
                        let val = self.eval_expr(program, case.body, &mut case_env, caller_context);
                        return val;
                    }
                }
                unreachable!()
            }
            Expr::RecordInitialization(type_id, items) => {
                let mut values: Vec<_> = Vec::with_capacity(items.len());
                for _ in 0..items.len() {
                    values.push(Value::new(ValueCore::Bool(false), expr_ty_id)); // dummy value
                }
                for item in items {
                    let value = self.eval_expr(program, item.expr_id, environment, caller_context);
                    values[item.index] = value;
                }
                return Value::new(ValueCore::Record(*type_id, values), expr_ty_id);
            }
            Expr::RecordUpdate(record_expr_id, updates) => {
                let value = self.eval_expr(program, *record_expr_id, environment, caller_context);
                if let ValueCore::Record(id, mut values) = value.core {
                    for update in updates {
                        if id == update.record_id {
                            for item in &update.items {
                                let value = self.eval_expr(
                                    program,
                                    item.expr_id,
                                    environment,
                                    caller_context,
                                );
                                values[item.index] = value;
                            }
                            return Value::new(ValueCore::Record(id, values), expr_ty_id);
                        }
                    }
                }
                unreachable!()
            }
            Expr::ClassFunctionCall(class_member_id, args) => {
                let member = program.class_members.get(class_member_id);
                let (class_member_type_id, class_arg_ty_id) = program
                    .class_member_types
                    .get(class_member_id)
                    .expect("untyped class member");
                let func_arg_types = self.get_func_arg_types(program, class_member_type_id);
                let callee_sub_context = self.get_callee_subtitution_context(
                    program,
                    &func_arg_types[..],
                    args,
                    caller_context,
                );
                let instance_selector_ty = program.to_concrete_type(class_arg_ty_id, &callee_sub_context);
                println!("instance selector {}", instance_selector_ty);
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
                            expr_ty_id,
                        );
                        let arg_values: Vec<_> = args
                            .iter()
                            .map(|e| self.eval_expr(program, *e, environment, caller_context))
                            .collect();
                        return self.call(callable, arg_values, program, expr_id, expr_ty_id);
                    }
                }
                unimplemented!()
            }
        }
    }

    fn call_extern(
        &self,
        module: &str,
        name: &str,
        environment: &mut Environment,
        program: &Program,
        current_expr: Option<ExprId>,
        instance: Option<String>,
        type_id: TypeId,
    ) -> Value {
        match (module, name) {
            (PRELUDE_NAME, "op_add") => {
                let l = environment.get_arg_by_index(0).core.as_int();
                let r = environment.get_arg_by_index(1).core.as_int();
                return Value::new(ValueCore::Int(l + r), type_id);
            }
            (PRELUDE_NAME, "op_sub") => {
                let l = environment.get_arg_by_index(0).core.as_int();
                let r = environment.get_arg_by_index(1).core.as_int();
                return Value::new(ValueCore::Int(l - r), type_id);
            }
            (PRELUDE_NAME, "op_mul") => {
                let l = environment.get_arg_by_index(0).core.as_int();
                let r = environment.get_arg_by_index(1).core.as_int();
                return Value::new(ValueCore::Int(l * r), type_id);
            }
            (PRELUDE_NAME, "op_lessthan") => {
                let l = environment.get_arg_by_index(0).core.as_int();
                let r = environment.get_arg_by_index(1).core.as_int();
                return Value::new(ValueCore::Bool(l < r), type_id);
            }
            (PRELUDE_NAME, "op_equals") => {
                let l = environment.get_arg_by_index(0).core.as_int();
                let r = environment.get_arg_by_index(1).core.as_int();
                return Value::new(ValueCore::Bool(l == r), type_id);
            }
            (PRELUDE_NAME, "op_notequals") => {
                let l = environment.get_arg_by_index(0).core.as_int();
                let r = environment.get_arg_by_index(1).core.as_int();
                return Value::new(ValueCore::Bool(l != r), type_id);
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
                return Value::new(ValueCore::Tuple(vec![]), type_id);
            }
            (PRELUDE_NAME, "print") => {
                let v = environment.get_arg_by_index(0).core.debug(program, false);
                print!("{}", v);
                return Value::new(ValueCore::Tuple(vec![]), type_id);
            }
            (PRELUDE_NAME, "println") => {
                let v = environment.get_arg_by_index(0).core.debug(program, false);
                println!("{}", v);
                return Value::new(ValueCore::Tuple(vec![]), type_id);
            }
            (PRELUDE_NAME, "show") => match instance {
                Some(instance_name) => match instance_name.as_ref() {
                    "ListShow" => {
                        let list = environment.get_arg_by_index(0);
                        return Value::new(
                            ValueCore::String(list.core.debug(program, false)),
                            type_id,
                        );
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
        caller_context: &SubstitutionContext,
    ) -> Value {
        let function = program.functions.get(&id);
        let function_type = program
            .function_types
            .get(&id)
            .expect("untyped func")
            .clone();
        match &function.info {
            FunctionInfo::NamedFunction(info) => match info.body {
                Some(body) => {
                    return self.eval_expr(program, body, environment, caller_context);
                }
                None => {
                    return self.call_extern(
                        &info.module,
                        &info.name,
                        environment,
                        program,
                        current_expr,
                        info.instance.clone(),
                        function_type,
                    );
                }
            },
            FunctionInfo::Lambda(info) => {
                return self.eval_expr(program, info.body, environment, caller_context);
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
                    function_type,
                );
            }
            FunctionInfo::RecordConstructor(info) => {
                let record = program.typedefs.get(&info.type_id).get_record();
                let mut values = Vec::new();
                for index in 0..record.fields.len() {
                    let v = environment.get_arg_by_index(index);
                    values.push(v);
                }
                return Value::new(ValueCore::Record(info.type_id, values), function_type);
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
                        return self.execute(program, *id, &mut environment, None, &sub_context);
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
