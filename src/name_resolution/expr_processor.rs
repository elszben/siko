use crate::constants::BuiltinOperator;
use crate::constants::PRELUDE_NAME;
use crate::ir::expr::Expr as IrExpr;
use crate::ir::expr::ExprId as IrExprId;
use crate::ir::expr::ExprInfo as IrExprInfo;
use crate::ir::function::Function as IrFunction;
use crate::ir::function::FunctionId as IrFunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::function::LambdaInfo;
use crate::ir::program::Program as IrProgram;
use crate::name_resolution::environment::Environment;
use crate::name_resolution::environment::NamedRef;
use crate::name_resolution::error::ResolverError;
use crate::name_resolution::item::Item;
use crate::name_resolution::lambda_helper::LambdaHelper;
use crate::name_resolution::module::Module;
use crate::syntax::expr::Expr;
use crate::syntax::expr::ExprId;
use crate::syntax::program::Program;
use std::collections::BTreeSet;

enum PathResolveResult {
    VariableRef(NamedRef),
    FunctionRef(IrFunctionId),
    Unknown(String),
    Ambiguous,
}

fn resolve_item_path(
    path: &str,
    module: &Module,
    environment: &Environment,
    lambda_helper: &mut LambdaHelper,
) -> PathResolveResult {
    let subs: Vec<_> = path.split(".").collect();
    if subs.len() == 1 {
        if let Some((named_ref, level)) = environment.get_ref(path) {
            let named_ref = lambda_helper.process_named_ref(named_ref.clone(), level);
            return PathResolveResult::VariableRef(named_ref);
        }
    }
    match module.imported_items.get(path) {
        Some(items) => {
            if items.len() > 1 {
                return PathResolveResult::Ambiguous;
            } else {
                let item = &items[0];
                match item.item {
                    Item::Function(_, ir_function_id) => {
                        return PathResolveResult::FunctionRef(ir_function_id);
                    }
                    _ => unimplemented!(),
                }
            }
        }
        None => {
            return PathResolveResult::Unknown(path.to_string());
        }
    }
}

fn add_expr(
    ir_expr: IrExpr,
    ast_id: ExprId,
    ir_program: &mut IrProgram,
    program: &Program,
) -> IrExprId {
    let expr_id = ir_program.get_expr_id();
    let location_id = program.get_expr_location(&ast_id);
    let expr_info = IrExprInfo::new(ir_expr, ast_id, location_id);
    ir_program.add_expr(expr_id, expr_info);
    expr_id
}

fn process_named_ref(
    named_ref: NamedRef,
    id: ExprId,
    ir_program: &mut IrProgram,
    program: &Program,
) -> IrExprId {
    let ir_expr = match named_ref {
        NamedRef::ExprValue(expr_ref) => IrExpr::ExprValue(expr_ref),
        NamedRef::FunctionArg(arg_ref) => IrExpr::ArgRef(arg_ref),
        NamedRef::LambdaCapturedExprValue(_, arg_ref) => IrExpr::LambdaCapturedArgRef(arg_ref),
        NamedRef::LambdaCapturedFunctionArg(_, arg_ref) => IrExpr::LambdaCapturedArgRef(arg_ref),
    };
    add_expr(ir_expr, id, ir_program, program)
}

pub fn process_expr(
    id: ExprId,
    program: &Program,
    module: &Module,
    environment: &mut Environment,
    ir_program: &mut IrProgram,
    errors: &mut Vec<ResolverError>,
    lambda_helper: &mut LambdaHelper,
) -> IrExprId {
    let expr = program.get_expr(&id);
    let location_id = program.get_expr_location(&id);
    println!("Processing expr {} {}", id, expr);
    match expr {
        Expr::Lambda(args, lambda_body) => {
            let ir_lambda_id = ir_program.get_function_id();
            let mut arg_names = BTreeSet::new();
            let mut conflicting_names = BTreeSet::new();
            let mut environment = Environment::child(environment);
            for (index, arg) in args.iter().enumerate() {
                if !arg_names.insert(arg.clone()) {
                    conflicting_names.insert(arg.clone());
                }
                environment.add_arg(arg.clone(), ir_lambda_id, index);
            }
            if !conflicting_names.is_empty() {
                let err = ResolverError::LambdaArgumentConflict(
                    conflicting_names.into_iter().collect(),
                    location_id.clone(),
                );
                errors.push(err);
            }
            let mut local_lambda_helper = LambdaHelper::new(
                environment.level(),
                lambda_helper.host_function(),
                lambda_helper.clone_counter(),
                ir_lambda_id,
            );

            let ir_lambda_body = process_expr(
                *lambda_body,
                program,
                module,
                &mut environment,
                ir_program,
                errors,
                &mut local_lambda_helper,
            );

            let lambda_info = LambdaInfo {
                body: ir_lambda_body,
                host_info: local_lambda_helper.host_function(),
                index: local_lambda_helper.get_lambda_index(),
            };

            let ir_function = IrFunction {
                id: ir_lambda_id,
                arg_count: args.len(),
                info: FunctionInfo::Lambda(lambda_info),
            };
            ir_program.add_function(ir_lambda_id, ir_function);

            let captured_lambda_args: Vec<_> = local_lambda_helper
                .captures()
                .into_iter()
                .map(|named_ref| process_named_ref(named_ref, id, ir_program, program))
                .collect();
            let ir_expr = IrExpr::LambdaFunction(ir_lambda_id, captured_lambda_args);
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::FunctionCall(id_expr_id, args) => {
            let ir_args: Vec<IrExprId> = args
                .iter()
                .map(|id| {
                    process_expr(
                        *id,
                        program,
                        module,
                        environment,
                        ir_program,
                        errors,
                        lambda_helper,
                    )
                })
                .collect();
            let id_expr = program.get_expr(id_expr_id);
            if let Expr::Path(path) = id_expr {
                match resolve_item_path(path, module, environment, lambda_helper) {
                    PathResolveResult::FunctionRef(n) => {
                        let ir_expr = IrExpr::StaticFunctionCall(n, ir_args);
                        return add_expr(ir_expr, id, ir_program, program);
                    }
                    PathResolveResult::VariableRef(named_ref) => {
                        let ir_id_expr_id =
                            process_named_ref(named_ref, *id_expr_id, ir_program, program);
                        let ir_expr = IrExpr::DynamicFunctionCall(ir_id_expr_id, ir_args);
                        return add_expr(ir_expr, id, ir_program, program);
                    }
                    PathResolveResult::Unknown(n) => {
                        let err = ResolverError::UnknownFunction(n, location_id);
                        errors.push(err);
                        let ir_expr = IrExpr::Tuple(vec![]);
                        return add_expr(ir_expr, id, ir_program, program);
                    }
                    PathResolveResult::Ambiguous => {
                        let err = ResolverError::AmbiguousName(path.clone(), location_id);
                        errors.push(err);
                        let ir_expr = IrExpr::Tuple(vec![]);
                        return add_expr(ir_expr, id, ir_program, program);
                    }
                }
            } else {
                if let Expr::Builtin(op) = id_expr {
                    if *op == BuiltinOperator::PipeForward {
                        assert_eq!(ir_args.len(), 2);
                        let left = ir_args[0];
                        let right = ir_args[1];
                        let ir_expr = IrExpr::DynamicFunctionCall(right, vec![left]);
                        return add_expr(ir_expr, id, ir_program, program);
                    } else {
                        let path =
                            format!("{}.op_{}", PRELUDE_NAME, format!("{:?}", op).to_lowercase());
                        match resolve_item_path(&path, module, environment, lambda_helper) {
                            PathResolveResult::FunctionRef(n) => {
                                let ir_expr = IrExpr::StaticFunctionCall(n, ir_args);
                                return add_expr(ir_expr, id, ir_program, program);
                            }
                            _ => panic!(
                                "Couldn't handle builtin function {}, missing {}?",
                                path.clone(),
                                PRELUDE_NAME
                            ),
                        }
                    }
                } else {
                    let id_expr = process_expr(
                        *id_expr_id,
                        program,
                        module,
                        environment,
                        ir_program,
                        errors,
                        lambda_helper,
                    );
                    let ir_expr = IrExpr::DynamicFunctionCall(id_expr, ir_args);
                    return add_expr(ir_expr, id, ir_program, program);
                }
            }
        }
        Expr::Builtin(_) => panic!("Builtinop reached!"),
        Expr::If(cond, true_branch, false_branch) => {
            let ir_cond = process_expr(
                *cond,
                program,
                module,
                environment,
                ir_program,
                errors,
                lambda_helper,
            );
            let ir_true_branch = process_expr(
                *true_branch,
                program,
                module,
                environment,
                ir_program,
                errors,
                lambda_helper,
            );
            let ir_false_branch = process_expr(
                *false_branch,
                program,
                module,
                environment,
                ir_program,
                errors,
                lambda_helper,
            );
            let ir_expr = IrExpr::If(ir_cond, ir_true_branch, ir_false_branch);
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::Tuple(items) => {
            let ir_items: Vec<IrExprId> = items
                .iter()
                .map(|id| {
                    process_expr(
                        *id,
                        program,
                        module,
                        environment,
                        ir_program,
                        errors,
                        lambda_helper,
                    )
                })
                .collect();
            let ir_expr = IrExpr::Tuple(ir_items);
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::Path(path) => match resolve_item_path(path, module, environment, lambda_helper) {
            PathResolveResult::FunctionRef(n) => {
                let ir_expr = IrExpr::StaticFunctionCall(n, vec![]);
                return add_expr(ir_expr, id, ir_program, program);
            }
            PathResolveResult::VariableRef(named_ref) => {
                return process_named_ref(named_ref, id, ir_program, program);
            }
            PathResolveResult::Unknown(n) => {
                let err = ResolverError::UnknownFunction(n, location_id);
                errors.push(err);
                let ir_expr = IrExpr::Tuple(vec![]);
                return add_expr(ir_expr, id, ir_program, program);
            }
            PathResolveResult::Ambiguous => {
                let err = ResolverError::AmbiguousName(path.clone(), location_id);
                errors.push(err);
                let ir_expr = IrExpr::Tuple(vec![]);
                return add_expr(ir_expr, id, ir_program, program);
            }
        },
        Expr::IntegerLiteral(v) => {
            let ir_expr = IrExpr::IntegerLiteral(v.clone());
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::FloatLiteral(v) => {
            let ir_expr = IrExpr::FloatLiteral(v.clone());
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::BoolLiteral(v) => {
            let ir_expr = IrExpr::BoolLiteral(v.clone());
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::StringLiteral(v) => {
            let ir_expr = IrExpr::StringLiteral(v.clone());
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::Do(items) => {
            let ir_items: Vec<IrExprId> = items
                .iter()
                .map(|id| {
                    process_expr(
                        *id,
                        program,
                        module,
                        environment,
                        ir_program,
                        errors,
                        lambda_helper,
                    )
                })
                .collect();
            let ir_expr = IrExpr::Do(ir_items);
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::Bind(name, expr_id) => {
            let ir_expr_id = process_expr(
                *expr_id,
                program,
                module,
                environment,
                ir_program,
                errors,
                lambda_helper,
            );
            environment.add_expr_value(name.clone(), ir_expr_id);
            let ir_expr = IrExpr::Bind(name.clone(), ir_expr_id);
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::FieldAccess(name, expr_id) => {
            let ir_expr_id = process_expr(
                *expr_id,
                program,
                module,
                environment,
                ir_program,
                errors,
                lambda_helper,
            );
            let ir_expr = IrExpr::Tuple(vec![]);
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::TupleFieldAccess(field_id, expr_id) => {
            let ir_expr_id = process_expr(
                *expr_id,
                program,
                module,
                environment,
                ir_program,
                errors,
                lambda_helper,
            );
            let ir_expr = IrExpr::Tuple(vec![]);
            return add_expr(ir_expr, id, ir_program, program);
        }
    }
}
