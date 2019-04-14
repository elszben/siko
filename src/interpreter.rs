use crate::constants;
use crate::constants::BuiltinOperator;
use crate::error::Error;
use crate::error::ErrorContext;
use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::expr::FunctionArgumentRef;
use crate::ir::function::Function;
use crate::ir::function::FunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::function::NamedFunctionInfo;
use crate::ir::program::Program;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Callable {
    function_id: FunctionId,
    values: Vec<Value>,
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Tuple(Vec<Value>),
    Callable(Callable),
}

impl Value {
    fn as_int(&self) -> i64 {
        match self {
            Value::Int(i) => *i,
            _ => unreachable!(),
        }
    }

    fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            _ => unreachable!(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{}", v),
            Value::Float(v) => write!(f, "{}", v),
            Value::Bool(v) => write!(f, "{}", v),
            Value::String(v) => write!(f, "{}", v),
            Value::Tuple(vs) => {
                let ss: Vec<_> = vs.iter().map(|v| format!("{}", v)).collect();
                write!(f, "({})", ss.join(", "))
            }
            Value::Callable(_) => write!(f, "<closure>"),
        }
    }
}

#[derive(Debug)]
struct Environment<'a> {
    function_id: FunctionId,
    args: Vec<Value>,
    variables: BTreeMap<ExprId, Value>,
    parent: Option<&'a Environment<'a>>,
}

impl<'a> Environment<'a> {
    fn new(function_id: FunctionId, args: Vec<Value>) -> Environment<'a> {
        Environment {
            function_id: function_id,
            args: args,
            variables: BTreeMap::new(),
            parent: None,
        }
    }

    fn add(&mut self, var: ExprId, value: Value) {
        self.variables.insert(var, value);
    }

    fn get_value(&self, var: &ExprId) -> Value {
        if let Some(value) = self.variables.get(var) {
            return value.clone();
        } else {
            if let Some(parent) = self.parent {
                parent.get_value(var)
            } else {
                panic!("Value {} not found", var);
            }
        }
    }

    fn child(
        parent: &'a Environment<'a>,
        args: Vec<Value>,
        function_id: FunctionId,
    ) -> Environment<'a> {
        Environment {
            function_id: function_id,
            args: args,
            variables: BTreeMap::new(),
            parent: Some(parent),
        }
    }

    fn block_child(parent: &'a Environment<'a>) -> Environment<'a> {
        Environment {
            function_id: parent.function_id,
            args: parent.args.clone(),
            variables: BTreeMap::new(),
            parent: Some(parent),
        }
    }

    fn get_arg(&self, arg_ref: &FunctionArgumentRef) -> Value {
        if self.function_id == arg_ref.id {
            return self.args[arg_ref.index].clone();
        } else {
            if let Some(parent) = self.parent {
                return parent.get_arg(arg_ref);
            } else {
                unreachable!()
            }
        }
    }
}

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
                let func_info = program.get_function(&callable.function_id);
                callable.values.extend(args);
                if func_info.arg_locations.len() > callable.values.len() {
                    Value::Callable(callable)
                } else {
                    assert_eq!(func_info.arg_locations.len(), (callable.values.len()));
                    let mut environment = Environment::new(callable.function_id, callable.values);
                    return self.execute(
                        program,
                        callable.function_id,
                        &mut environment,
                        Some(expr_id),
                    );
                }
            }
            _ => unreachable!(),
        }
    }

    fn eval_expr(
        &mut self,
        program: &Program,
        expr_id: ExprId,
        environment: &mut Environment,
    ) -> Value {
        let expr = program.get_expr(&expr_id);
        // println!("Eval {}", expr);
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
                let l = environment.args[0].as_int();
                let r = environment.args[1].as_int();
                return Value::Int(l + r);
            }
            ("Prelude", "op_sub") => {
                let l = environment.args[0].as_int();
                let r = environment.args[1].as_int();
                return Value::Int(l - r);
            }
            ("Prelude", "op_mul") => {
                let l = environment.args[0].as_int();
                let r = environment.args[1].as_int();
                return Value::Int(l * r);
            }
            ("Prelude", "op_lessthan") => {
                let l = environment.args[0].as_int();
                let r = environment.args[1].as_int();
                return Value::Bool(l < r);
            }
            ("Prelude", "op_equals") => {
                let l = environment.args[0].as_int();
                let r = environment.args[1].as_int();
                return Value::Bool(l == r);
            }
            ("Std.Util", "assert") => {
                let v = environment.args[0].as_bool();
                if !v {
                    let current_expr = current_expr.expect("No current expr");
                    let location_id = program.get_expr_location(&current_expr);
                    let err = Error::RuntimeError(format!("Assertion failed"), location_id);
                    err.report_error(&self.error_context);
                    panic!("Abort not implemented");
                }
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
                        let mut environment = Environment::new(*id, vec![]);
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
