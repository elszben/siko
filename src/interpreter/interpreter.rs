use crate::constants;
use crate::error::Error;
use crate::error::ErrorContext;
use crate::interpreter::environment::Environment;
use crate::interpreter::value::Callable;
use crate::interpreter::value::Value;
use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::function::FunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::pattern::Pattern;
use crate::ir::pattern::PatternId;
use crate::ir::program::Program;

pub struct Interpreter<'a> {
    error_context: ErrorContext<'a>,
}

impl<'a> Interpreter<'a> {
    pub fn new(error_context: ErrorContext<'a>) -> Interpreter<'a> {
        Interpreter {
            error_context: error_context,
        }
    }

    fn call(
        &mut self,
        callable: Value,
        args: Vec<Value>,
        program: &Program,
        expr_id: ExprId,
    ) -> Value {
        match callable {
            Value::Callable(mut callable) => {
                callable.values.extend(args);
                loop {
                    let func_info = program.get_function(&callable.function_id);
                    let needed_arg_count =
                        func_info.arg_locations.len() + func_info.implicit_arg_count;
                    if needed_arg_count > callable.values.len() {
                        return Value::Callable(callable);
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
                        );
                        if !rest.is_empty() {
                            if let Value::Callable(new_callable) = result {
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
    ) -> bool {
        let pattern = program.get_pattern(pattern_id);
        match pattern {
            Pattern::Binding(_) => {
                environment.add(*pattern_id, value.clone());
                return true;
            }
            Pattern::Tuple(ids) => match value {
                Value::Tuple(vs) => {
                    for (index, id) in ids.iter().enumerate() {
                        let v = &vs[index];
                        if !self.match_pattern(id, v, program, environment) {
                            return false;
                        }
                    }
                    return true;
                }
                _ => {
                    return false;
                }
            },
            Pattern::Record(p_type_id, p_ids) => match value {
                Value::Record(type_id, vs) => {
                    if type_id == p_type_id {
                        for (index, p_id) in p_ids.iter().enumerate() {
                            let v = &vs[index];
                            if !self.match_pattern(p_id, v, program, environment) {
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
            Pattern::Variant(p_type_id, p_index, p_ids) => match value {
                Value::Variant(type_id, index, vs) => {
                    if type_id == p_type_id && index == p_index {
                        for (index, p_id) in p_ids.iter().enumerate() {
                            let v = &vs[index];
                            if !self.match_pattern(p_id, v, program, environment) {
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
                if self.match_pattern(id, value, program, environment) {
                    let guard_value = self.eval_expr(program, *guard_expr_id, environment);
                    return guard_value.as_bool();
                } else {
                    return false;
                }
            }
            Pattern::Wildcard => {
                return true;
            }
            Pattern::IntegerLiteral(p_v) => {
                let r = match value {
                    Value::Int(v) => p_v == v,
                    _ => false,
                };
                return r;
            }
            Pattern::FloatLiteral(p_v) => {
                let r = match value {
                    Value::Float(v) => p_v == v,
                    _ => false,
                };
                return r;
            }
            Pattern::StringLiteral(p_v) => {
                let r = match value {
                    Value::String(v) => p_v == v,
                    _ => false,
                };
                return r;
            }
            Pattern::BoolLiteral(p_v) => {
                let r = match value {
                    Value::Bool(v) => p_v == v,
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
    ) -> Value {
        let expr = program.get_expr(&expr_id);
        //println!("Eval {} {}", expr_id, expr);
        match expr {
            Expr::IntegerLiteral(v) => Value::Int(*v),
            Expr::StringLiteral(v) => Value::String(v.clone()),
            Expr::FloatLiteral(v) => Value::Float(v.clone()),
            Expr::BoolLiteral(v) => Value::Bool(v.clone()),
            Expr::ArgRef(arg_ref) => {
                return environment.get_arg(arg_ref);
            }
            Expr::StaticFunctionCall(function_id, args) => {
                let callable = Value::Callable(Callable {
                    function_id: *function_id,
                    values: vec![],
                });
                let arg_values: Vec<_> = args
                    .iter()
                    .map(|arg| self.eval_expr(program, *arg, environment))
                    .collect();
                return self.call(callable, arg_values, program, expr_id);
            }
            Expr::DynamicFunctionCall(function_expr_id, args) => {
                let function_expr_id = self.eval_expr(program, *function_expr_id, environment);
                let arg_values: Vec<_> = args
                    .iter()
                    .map(|arg| self.eval_expr(program, *arg, environment))
                    .collect();
                return self.call(function_expr_id, arg_values, program, expr_id);
            }
            Expr::Do(exprs) => {
                let mut environment = Environment::block_child(environment);
                let mut result = Value::Bool(false);
                for expr in exprs {
                    result = self.eval_expr(program, *expr, &mut environment);
                }
                return result;
            }
            Expr::Bind(pattern_id, expr_id) => {
                let value = self.eval_expr(program, *expr_id, environment);
                let r = self.match_pattern(pattern_id, &value, program, environment);
                assert!(r);
                return Value::Tuple(vec![]);
            }
            Expr::ExprValue(_, pattern_id) => {
                return environment.get_value(pattern_id);
            }
            Expr::If(cond, true_branch, false_branch) => {
                let cond_value = self.eval_expr(program, *cond, environment);
                if cond_value.as_bool() {
                    return self.eval_expr(program, *true_branch, environment);
                } else {
                    return self.eval_expr(program, *false_branch, environment);
                }
            }
            Expr::Tuple(exprs) => {
                let values: Vec<_> = exprs
                    .iter()
                    .map(|e| self.eval_expr(program, *e, environment))
                    .collect();
                return Value::Tuple(values);
            }
            Expr::TupleFieldAccess(index, tuple) => {
                let tuple_value = self.eval_expr(program, *tuple, environment);
                if let Value::Tuple(t) = tuple_value {
                    return t[*index].clone();
                } else {
                    unreachable!()
                }
            }
            Expr::Formatter(fmt, args) => {
                let subs: Vec<_> = fmt.split("{}").collect();
                let values: Vec<_> = args
                    .iter()
                    .map(|e| self.eval_expr(program, *e, environment))
                    .collect();
                let mut result = String::new();
                for (index, sub) in subs.iter().enumerate() {
                    result += sub;
                    if values.len() > index {
                        result += &values[index].debug(program, false);
                    }
                }
                return Value::String(result);
            }
            Expr::FieldAccess(infos, record_expr) => {
                let record = self.eval_expr(program, *record_expr, environment);
                let (id, values) = if let Value::Record(id, values) = record {
                    (id, values)
                } else {
                    unreachable!()
                };
                for info in infos {
                    if info.record_id != id {
                        continue;
                    }
                    return values[info.index].clone();
                }
                unreachable!()
            }
            Expr::CaseOf(body, cases) => {
                let case_value = self.eval_expr(program, *body, environment);
                for case in cases {
                    let mut case_env = Environment::block_child(environment);
                    if self.match_pattern(&case.pattern_id, &case_value, program, &mut case_env) {
                        let val = self.eval_expr(program, case.body, &mut case_env);
                        return val;
                    }
                }
                unreachable!()
            }
            Expr::RecordInitialization(type_id, items) => {
                let mut values: Vec<_> = Vec::with_capacity(items.len());
                for _ in 0..items.len() {
                    values.push(Value::Bool(false));
                }
                for item in items {
                    let value = self.eval_expr(program, item.expr_id, environment);
                    values[item.index] = value;
                }
                return Value::Record(*type_id, values);
            }
            Expr::RecordUpdate(expr_id, pattern_id, items) => unimplemented!(),
        }
    }

    fn call_extern(
        &self,
        module: &str,
        name: &str,
        environment: &mut Environment,
        program: &Program,
        current_expr: Option<ExprId>,
    ) -> Value {
        match (module, name) {
            ("Prelude", "op_add") => {
                let l = environment.get_arg_by_index(0).as_int();
                let r = environment.get_arg_by_index(1).as_int();
                return Value::Int(l + r);
            }
            ("Prelude", "op_sub") => {
                let l = environment.get_arg_by_index(0).as_int();
                let r = environment.get_arg_by_index(1).as_int();
                return Value::Int(l - r);
            }
            ("Prelude", "op_mul") => {
                let l = environment.get_arg_by_index(0).as_int();
                let r = environment.get_arg_by_index(1).as_int();
                return Value::Int(l * r);
            }
            ("Prelude", "op_lessthan") => {
                let l = environment.get_arg_by_index(0).as_int();
                let r = environment.get_arg_by_index(1).as_int();
                return Value::Bool(l < r);
            }
            ("Prelude", "op_equals") => {
                let l = environment.get_arg_by_index(0).as_int();
                let r = environment.get_arg_by_index(1).as_int();
                return Value::Bool(l == r);
            }
            ("Prelude", "op_notequals") => {
                let l = environment.get_arg_by_index(0).as_int();
                let r = environment.get_arg_by_index(1).as_int();
                return Value::Bool(l != r);
            }
            ("Std.Util", "assert") => {
                let v = environment.get_arg_by_index(0).as_bool();
                if !v {
                    let current_expr = current_expr.expect("No current expr");
                    let location_id = program.get_expr_location(&current_expr);
                    let err = Error::RuntimeError(format!("Assertion failed"), location_id);
                    err.report_error(&self.error_context);
                    panic!("Abort not implemented");
                }
                return Value::Tuple(vec![]);
            }
            ("Prelude", "print") => {
                let v = environment.get_arg_by_index(0).debug(program, false);
                print!("{}", v);
                return Value::Tuple(vec![]);
            }
            ("Prelude", "println") => {
                let v = environment.get_arg_by_index(0).debug(program, false);
                println!("{}", v);
                return Value::Tuple(vec![]);
            }
            ("Prelude", "empty") => {
                return Value::List(vec![]);
            }
            ("Prelude", "insert") => {
                let list = environment.get_arg_by_index(0);
                let item = environment.get_arg_by_index(1);
                if let Value::List(mut vs) = list {
                    vs.push(item);
                    return Value::List(vs);
                } else {
                    unreachable!();
                }
            }
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
    ) -> Value {
        let function = program.get_function(&id);
        match &function.info {
            FunctionInfo::NamedFunction(info) => match info.body {
                Some(body) => {
                    return self.eval_expr(program, body, environment);
                }
                None => {
                    return self.call_extern(
                        &info.module,
                        &info.name,
                        environment,
                        program,
                        current_expr,
                    );
                }
            },
            FunctionInfo::Lambda(info) => {
                return self.eval_expr(program, info.body, environment);
            }
            FunctionInfo::VariantConstructor(info) => {
                let adt = program.get_adt(&info.type_id);
                let variant = &adt.variants[info.index];
                let mut values = Vec::new();
                for index in 0..variant.items.len() {
                    let v = environment.get_arg_by_index(index);
                    values.push(v);
                }
                return Value::Variant(info.type_id, info.index, values);
            }
            FunctionInfo::RecordConstructor(info) => {
                let record = program.get_record(&info.type_id);
                let mut values = Vec::new();
                for index in 0..record.fields.len() {
                    let v = environment.get_arg_by_index(index);
                    values.push(v);
                }
                return Value::Record(info.type_id, values);
            }
        }
    }

    pub fn run(&mut self, program: &Program) -> Value {
        for (id, function) in &program.functions {
            match &function.info {
                FunctionInfo::NamedFunction(info) => {
                    if info.module == constants::MAIN_MODULE
                        && info.name == constants::MAIN_FUNCTION
                    {
                        let mut environment = Environment::new(*id, vec![], 0);
                        return self.execute(program, *id, &mut environment, None);
                    }
                }
                _ => {}
            }
        }

        panic!(
            "Cannot find function {} in module {}",
            constants::MAIN_FUNCTION,
            constants::MAIN_MODULE
        );
    }
}
