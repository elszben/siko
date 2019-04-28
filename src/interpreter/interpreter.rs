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
use crate::ir::program::Program;
use std::fmt::Write;

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

    fn eval_expr(
        &mut self,
        program: &Program,
        expr_id: ExprId,
        environment: &mut Environment,
    ) -> Value {
        let expr = program.get_expr(&expr_id);
        println!("Eval {}", expr);
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
            Expr::Bind(_, id) => {
                let value = self.eval_expr(program, *id, environment);
                environment.add(*id, value);
                return Value::Tuple(vec![]);
            }
            Expr::ExprValue(ref_expr_id) => {
                return environment.get_value(ref_expr_id);
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
                    write!(result, "{}", sub).unwrap();
                    if values.len() > index {
                        write!(result, "{}", values[index]).unwrap();
                    }
                }
                return Value::String(result);
            }
            _ => panic!("{} eval is not implemented", expr),
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
            ("Std.IO", "print") => {
                let v = environment.get_arg_by_index(0).as_string();
                print!("{}", v);
                return Value::Tuple(vec![]);
            }
            ("Std.IO", "println") => {
                let v = environment.get_arg_by_index(0).as_string();
                println!("{}", v);
                return Value::Tuple(vec![]);
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
            _ => unimplemented!(),
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
