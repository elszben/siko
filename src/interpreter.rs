use crate::constants;
use crate::constants::BuiltinOperator;
use crate::error::Error;
use crate::ir::Expr;
use crate::ir::Function;
use crate::ir::FunctionBody;
use crate::ir::FunctionId;
use crate::ir::Program;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct CaptureList {
    captures: Vec<(String, Value)>,
}

impl CaptureList {
    fn new() -> CaptureList {
        CaptureList {
            captures: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.captures.is_empty()
    }

    fn add(&mut self, name: String, value: Value) {
        self.captures.push((name, value));
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Tuple(Vec<Value>),
    Callable(FunctionId, CaptureList),
}

impl Value {
    fn eval_binary_op(&self, op: BuiltinOperator, other: &Value) -> Result<Value, Error> {
        match op {
            BuiltinOperator::Mul => match self {
                Value::Int(v1) => match other {
                    Value::Int(v2) => {
                        let r = *v1 * *v2;
                        return Ok(Value::Int(r));
                    }
                    _ => {}
                },
                _ => {}
            },
            BuiltinOperator::Div => match self {
                Value::Int(v1) => match other {
                    Value::Int(v2) => {
                        let r = *v1 / *v2;
                        return Ok(Value::Int(r));
                    }
                    _ => {}
                },
                _ => {}
            },
            BuiltinOperator::And => match self {
                Value::Bool(v1) => match other {
                    Value::Bool(v2) => {
                        let r = *v1 && *v2;
                        return Ok(Value::Bool(r));
                    }
                    _ => {}
                },
                _ => {}
            },
            BuiltinOperator::Or => match self {
                Value::Bool(v1) => match other {
                    Value::Bool(v2) => {
                        let r = *v1 || *v2;
                        return Ok(Value::Bool(r));
                    }
                    _ => {}
                },
                _ => {}
            },
            BuiltinOperator::Equals => match self {
                Value::Bool(v1) => match other {
                    Value::Bool(v2) => {
                        let r = *v1 == *v2;
                        return Ok(Value::Bool(r));
                    }
                    _ => {}
                },
                Value::Int(v1) => match other {
                    Value::Int(v2) => {
                        let r = *v1 == *v2;
                        return Ok(Value::Bool(r));
                    }
                    _ => {}
                },
                _ => {}
            },
            BuiltinOperator::NotEquals => match self {
                Value::Bool(v1) => match other {
                    Value::Bool(v2) => {
                        let r = *v1 != *v2;
                        return Ok(Value::Bool(r));
                    }
                    _ => {}
                },
                _ => {}
            },
            BuiltinOperator::LessThan => match self {
                Value::Int(v1) => match other {
                    Value::Int(v2) => {
                        let r = *v1 < *v2;
                        return Ok(Value::Bool(r));
                    }
                    _ => {}
                },
                _ => {}
            },

            _ => {}
        }
        let err = format!(
            "Unimplemented binary op {:?} for {:?} and {:?}",
            op, self, other
        );
        return Err(Error::runtime_err(err));
    }

    fn get_bool(&self) -> bool {
        if let Value::Bool(b) = self {
            return *b;
        }
        panic!("{} is not a bool", self);
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
            Value::Callable(v, captures) => write!(f, "{:?} {:?}", v, captures),
        }
    }
}

struct Environment<'a> {
    variables: BTreeMap<String, Value>,
    parent: Option<&'a Environment<'a>>,
}

impl<'a> Environment<'a> {
    fn new() -> Environment<'a> {
        Environment {
            variables: BTreeMap::new(),
            parent: None,
        }
    }

    fn add(&mut self, var: String, value: Value) {
        self.variables.insert(var, value);
    }

    fn get_value(&self, var: &str) -> Value {
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

    fn child(parent: &'a Environment<'a>) -> Environment<'a> {
        Environment {
            variables: BTreeMap::new(),
            parent: Some(parent),
        }
    }
}

#[derive(Debug)]
pub struct Interpreter {}

impl Interpreter {
    pub fn new() -> Interpreter {
        Interpreter {}
    }

    fn call_callable(
        &self,
        callable: &Value,
        arg_values: &[Value],
        program: &Program,
        environment: &Environment,
    ) -> Result<Value, Error> {
        let (id, captures) = if let Value::Callable(id, captures) = callable {
            (id, captures)
        } else {
            return Err(Error::runtime_err(format!("Cannot call {:?}", callable)));
        };
        match id {
            FunctionId::NamedFunctionId(function_id) => {
                let function =
                    program.get_function(&function_id.module_name, &function_id.function_name);
                let mut environment = Environment::child(environment);
                if arg_values.len() != function.args.len() {
                    let err = format!(
                        "Unexpected argument count {} != {}",
                        arg_values.len(),
                        function.args.len()
                    );
                    return Err(Error::runtime_err(err));
                }
                for (arg_value, arg_name) in arg_values.into_iter().zip(function.args.iter()) {
                    environment.add(arg_name.0.clone(), arg_value.clone());
                }
                return self.execute(program, function, &mut environment);
            }
            /*
            FunctionId::Builtin(op) => {
                if *op == BuiltinOperator::Not {
                    let v = arg_values[0].get_bool();
                    return Ok(Value::Bool(!v));
                } else if *op == BuiltinOperator::PipeForward {
                    let value = arg_values[0].clone();
                    let callable = &arg_values[1];
                    return self.call_callable(callable, &[value], program, environment);
                }
                let left = &arg_values[0];
                let right = &arg_values[1];
                return left.eval_binary_op(*op, right);
            }
            */
            FunctionId::Lambda(name) => {
                let function = program.get_lambda(name);
                let mut environment = Environment::child(environment);
                if arg_values.len() != function.args.len() {
                    let err = format!(
                        "Unexpected argument count for lambda {} != {}",
                        arg_values.len(),
                        function.args.len()
                    );
                    return Err(Error::runtime_err(err));
                }
                for (arg_value, arg_name) in arg_values.into_iter().zip(function.args.iter()) {
                    environment.add(arg_name.0.clone(), arg_value.clone());
                }
                for (name, value) in captures.captures.clone() {
                    environment.add(name, value);
                }
                return self.execute(program, function, &mut environment);
            }
        }
    }

    fn evaluate(
        &self,
        program: &Program,
        expr: &Expr,
        environment: &mut Environment,
    ) -> Result<Value, Error> {
        match expr {
            Expr::Lambda(capture_names, function_id) => {
                let mut captures = CaptureList::new();
                for name in capture_names {
                    let value = environment.get_value(name);
                    captures.add(name.clone(), value);
                }
                return Ok(Value::Callable(function_id.clone(), captures));
            }
            Expr::FunctionCall(function_id_expr, args) => {
                let callable = self.evaluate(program, function_id_expr, environment)?;
                let mut arg_values = Vec::new();
                for arg in args.iter() {
                    let value = self.evaluate(program, arg, environment)?;
                    arg_values.push(value);
                }
                return self.call_callable(&callable, &arg_values[..], program, environment);
            }
            Expr::If(cond, true_branch, false_branch) => {
                let cond_value = self.evaluate(program, cond, environment)?;
                let v = cond_value.get_bool();
                if v {
                    return self.evaluate(program, true_branch, environment);
                } else {
                    return self.evaluate(program, false_branch, environment);
                }
            }
            Expr::IntegerLiteral(value, _) => {
                return Ok(Value::Int(value.clone()));
            }
            Expr::BoolLiteral(value, _) => {
                return Ok(Value::Bool(value.clone()));
            }
            Expr::StringLiteral(value, _) => {
                return Ok(Value::String(value.clone()));
            }
            Expr::Do(exprs) => {
                let mut environment = Environment::child(environment);
                let mut last = Value::Tuple(vec![]);
                for expr in exprs.iter() {
                    last = self.evaluate(program, expr, &mut environment)?;
                    match &last {
                        Value::Callable(id, captures) => {
                            if let FunctionId::NamedFunctionId(id) = id {
                                let function =
                                    program.get_function(&id.module_name, &id.function_name);
                                if function.args.is_empty() {
                                    // no argument functions cannot be lambda functions
                                    assert!(captures.is_empty());
                                    last = self.execute(program, function, &mut environment)?;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                return Ok(last);
            }
            Expr::Bind((name, _), expr) => {
                let value = self.evaluate(program, expr, environment)?;
                environment.add(name.clone(), value);
                return Ok(Value::Tuple(vec![]));
            }
            Expr::VariableRef(name, _) => {
                let value = environment.get_value(name);
                return Ok(value);
            }
            Expr::FunctionRef(function_id, _) => {
                return Ok(Value::Callable(function_id.clone(), CaptureList::new()));
            }
            _ => panic!("Unimplemented expr evaluation {:?}", expr),
        }
    }

    fn execute(
        &self,
        program: &Program,
        function: &Function,
        environment: &mut Environment,
    ) -> Result<Value, Error> {
        match &function.body {
            FunctionBody::Expr(expr) => {
                return self.evaluate(program, expr, environment);
            }
            FunctionBody::Extern => {
                match function.module.as_ref() {
                    "Std.IO" => match function.name.as_ref() {
                        "print" => {
                            let value = environment.get_value("msg");
                            println!("{}", value);
                            return Ok(Value::Tuple(vec![]));
                        }
                        _ => {}
                    },
                    "Std.Util" => match function.name.as_ref() {
                        "assert" => {
                            let v = environment.get_value("value");
                            if !v.get_bool() {
                                panic!("Assertion failed");
                            }
                            return Ok(Value::Tuple(vec![]));
                        }
                        _ => {}
                    },
                    "Prelude" => match function.name.as_ref() {
                        "op_add" => {
                            let x = environment.get_value("x");
                            let y = environment.get_value("y");
                            match x {
                                Value::Int(v1) => match y {
                                    Value::Int(v2) => {
                                        let r = v1 + v2;
                                        return Ok(Value::Int(r));
                                    }
                                    _ => {}
                                },
                                _ => {}
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
                panic!("Unimplemented extern function {}", function.name);
            }
        }
    }

    pub fn run(&self, program: &Program) -> Result<Value, Error> {
        if !program.is_valid() {
            return Err(Error::runtime_err(format!(
                "Cannot find function {} in module {}",
                constants::MAIN_FUNCTION,
                constants::MAIN_MODULE
            )));
        }
        let main_function = program.get_function(constants::MAIN_MODULE, constants::MAIN_FUNCTION);
        let mut environment = Environment::new();
        return self.execute(program, main_function, &mut environment);
    }
}
