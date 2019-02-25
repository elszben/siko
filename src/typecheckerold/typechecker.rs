use super::constraint_store::ConstraintStore;
use super::environment::Environment;
use super::function_store::FunctionInfo;
use super::function_store::FunctionStore;
use super::function_type::FunctionType;
use super::type_constraint::FunctionCall;
use super::type_constraint::FunctionHeader;
use super::type_constraint::TypeConstraint;
use super::type_store::TypeStore;
use super::type_variable::TypeVariable;
use super::types::Type;
use crate::error::Error;
use crate::ir::Expr;
use crate::ir::Function;
use crate::ir::FunctionBody;
use crate::ir::FunctionId;
use crate::ir::NamedFunctionId;
use crate::ir::Program;
use crate::ir::TypeSignature;

use std::collections::BTreeMap;
use std::collections::BTreeSet;

pub struct Typechecker {}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {}
    }

    fn process_expr(
        &self,
        expr: &Expr,
        type_store: &mut TypeStore,
        constraint_store: &mut ConstraintStore,
        environment: &mut Environment,
        used_functions: &mut BTreeSet<FunctionId>,
    ) -> Result<(TypeVariable, Type), Error> {
        match expr {
            Expr::IntegerLiteral(_, _) => {
                let ty = Type::Int;
                let var = type_store.add_var(ty.clone());
                return Ok((var, ty));
            }
            Expr::StringLiteral(_, _) => {
                let ty = Type::String;
                let var = type_store.add_var(ty.clone());
                return Ok((var, ty));
            }
            Expr::VariableRef(name, _) => {
                return Ok(environment.get_value(name));
            }
            Expr::Tuple(exprs) => {
                let mut subexpr_vars = Vec::new();
                for e in exprs {
                    let (var, _) = self.process_expr(
                        e,
                        type_store,
                        constraint_store,
                        environment,
                        used_functions,
                    )?;
                    subexpr_vars.push(Type::TypeVar(var));
                }
                let ty = Type::Tuple(subexpr_vars);
                let var = type_store.add_var(ty.clone());
                return Ok((var, ty));
            }
            Expr::FunctionCall(id_expr, args) => match id_expr.as_ref() {
                Expr::FunctionRef(id, _) => {
                    let ty = Type::TypeArgument(type_store.get_unique_type_arg());
                    let return_type_var = type_store.add_var(ty.clone());
                    let mut call_args = Vec::new();
                    for arg in args.iter() {
                        let (var, _) = self.process_expr(
                            arg,
                            type_store,
                            constraint_store,
                            environment,
                            used_functions,
                        )?;
                        call_args.push(var);
                    }
                    let call = FunctionCall {
                        function_id: id.clone(),
                        args: call_args,
                        return_type: return_type_var,
                        callee_vars: Vec::new(),
                    };
                    used_functions.insert(id.clone());
                    let c = TypeConstraint::FunctionCall(call);
                    constraint_store.add(c);
                    return Ok((return_type_var, ty));
                }
                _ => panic!("Closures are not implemented"),
            },
            _ => panic!("Not implemented {:?}", expr),
        }
    }

    fn process_function_type(
        &self,
        function_type: &FunctionType,
        function: &Function,
    ) -> Result<(), Error> {
        if function_type.get_arg_count() != function.args.len() {
            let err = Error::typecheck_err(format!(
                "Invalid argument count in function type, must match function"
            ));
            return Err(err);
        }
        Ok(())
    }

    fn collect_constraints(
        &self,
        function: &Function,
        function_store: &mut FunctionStore,
    ) -> Result<(), Error> {
        println!("Typecheck: processing function '{}'", function.name);
        let mut environment = Environment::new();

        let function_id = function.get_function_id();

        let func_info = function_store.get_function_info_mut(&function_id);

        if func_info.name != function.name {
            let err = Error::typecheck_err(format!(
                "Mismatching name in function type, must be same as function"
            ));
            return Err(err);
        }

        if let Some(func_type) = &func_info.function_type {
            self.process_function_type(func_type, function)?;
        }

        let mut args = Vec::new();
        for (arg, _) in function.args.iter() {
            let ty = Type::TypeArgument(func_info.type_store.get_unique_type_arg());
            let type_var = func_info.type_store.add_var(ty.clone());
            args.push(type_var.clone());
            environment.add(arg.clone(), type_var, ty);
        }

        let mut used_functions = BTreeSet::new();

        let body_return_type_var = match &function.body {
            FunctionBody::Expr(expr) => {
                let (var, _) = self.process_expr(
                    expr,
                    &mut func_info.type_store,
                    &mut func_info.constraint_store,
                    &mut environment,
                    &mut used_functions,
                )?;
                Some(var)
            }
            FunctionBody::Extern => None,
        };

        let function_header = FunctionHeader {
            function_type: func_info.function_type.clone(),
            arguments: args,
            body_return_type: body_return_type_var,
            used_functions: used_functions.into_iter().collect(),
        };

        func_info
            .constraint_store
            .set_function_header(function_header);

        Ok(())
    }

    fn convert_type(ir_type: &TypeSignature, arg_map: &mut BTreeMap<String, usize>) -> Type {
        match ir_type {
            TypeSignature::Bool => Type::Bool,
            TypeSignature::Int => Type::Int,
            TypeSignature::String => Type::String,
            TypeSignature::Nothing => Type::Nothing,
            TypeSignature::Tuple(types) => Type::Tuple(
                types
                    .iter()
                    .map(|t| Typechecker::convert_type(t, arg_map))
                    .collect(),
            ),
            TypeSignature::Function(types) => {
                let types = types
                    .iter()
                    .map(|t| Typechecker::convert_type(t, arg_map))
                    .collect();
                Type::Function(FunctionType::new(types))
            }
            TypeSignature::Invalid(..) => unreachable!(),
            TypeSignature::TypeArgument(n, _) => {
                let len = arg_map.len();
                let index = arg_map.entry(n.clone()).or_insert(len);
                Type::TypeArgument(*index)
            }
        }
    }

    fn register_function(
        &self,
        function_store: &mut FunctionStore,
        ir_function: &Function,
    ) -> Result<(), Error> {
        let function_id = ir_function.get_function_id();
        let function_info = match &ir_function.func_type {
            Some(ir_func_type) => {
                let mut arg_map = BTreeMap::new();
                let type_signature =
                    Typechecker::convert_type(&ir_func_type.type_signature, &mut arg_map);
                if let Type::Function(func_type) = type_signature {
                    let function_info =
                        FunctionInfo::new(ir_func_type.name.0.clone(), Some(func_type));
                    function_info
                } else {
                    let func_type = FunctionType::new(vec![type_signature]);
                    let function_info =
                        FunctionInfo::new(ir_func_type.name.0.clone(), Some(func_type));
                    function_info
                }
            }
            None => FunctionInfo::new(ir_function.name.clone(), None),
        };
        println!(
            "Registering function {:?} with type {}",
            function_id, function_info
        );
        function_store.add(function_id, function_info);
        Ok(())
    }

    pub fn check(&self, program: &Program) -> Result<(), Error> {
        let mut function_store = FunctionStore::new();

        for module in program.modules.values() {
            for function in module.functions.values() {
                self.register_function(&mut function_store, &function)?;
            }
        }

        for lambda in program.lambdas.values() {
            self.register_function(&mut function_store, &lambda)?;
        }

        for module in program.modules.values() {
            for function in module.functions.values() {
                self.collect_constraints(&function, &mut function_store)?;
            }
        }

        return function_store.process();
    }
}
