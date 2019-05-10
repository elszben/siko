use crate::constants::BuiltinOperator;
use crate::constants::PRELUDE_NAME;
use crate::ir::expr::Expr as IrExpr;
use crate::ir::expr::ExprId as IrExprId;
use crate::ir::expr::ExprInfo as IrExprInfo;
use crate::ir::expr::FieldAccessInfo;
use crate::ir::function::Function as IrFunction;
use crate::ir::function::FunctionId as IrFunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::function::LambdaInfo;
use crate::ir::program::Program as IrProgram;
use crate::ir::types::TypeDef;
use crate::location_info::item::LocationId;
use crate::name_resolution::environment::Environment;
use crate::name_resolution::error::ResolverError;
use crate::name_resolution::item::DataMember;
use crate::name_resolution::item::Item;
use crate::name_resolution::lambda_helper::LambdaHelper;
use crate::name_resolution::module::Module;
use crate::syntax::expr::Expr;
use crate::syntax::expr::ExprId;
use crate::syntax::program::Program;
use std::collections::BTreeSet;

enum PathResolveResult {
    VariableRef(IrExprId),
    FunctionRef(IrFunctionId),
}

fn resolve_item_path(
    path: &str,
    module: &Module,
    environment: &Environment,
    lambda_helper: LambdaHelper,
    program: &Program,
    ir_program: &mut IrProgram,
    id: ExprId,
    errors: &mut Vec<ResolverError>,
    location_id: LocationId,
) -> PathResolveResult {
    let subs: Vec<_> = path.split(".").collect();
    if subs.len() == 1 {
        if let Some((named_ref, level)) = environment.get_ref(path) {
            let ir_expr = lambda_helper.process_named_ref(named_ref.clone(), level);
            let ir_expr_id = add_expr(ir_expr, id, ir_program, program);
            return PathResolveResult::VariableRef(ir_expr_id);
        }
    }
    match module.imported_items.get(path) {
        Some(items) => {
            if items.len() > 1 {
                let err = ResolverError::AmbiguousName(path.to_string(), location_id);
                errors.push(err);
                let ir_expr = IrExpr::Tuple(vec![]);
                let ir_expr_id = add_expr(ir_expr, id, ir_program, program);
                return PathResolveResult::VariableRef(ir_expr_id);
            } else {
                let item = &items[0];
                match item.item {
                    Item::Function(_, ir_function_id) => {
                        return PathResolveResult::FunctionRef(ir_function_id);
                    }
                    Item::Record(_, ir_typedef_id) => {
                        let ir_typedef = ir_program
                            .typedefs
                            .get(&ir_typedef_id)
                            .expect("Record not found");
                        if let TypeDef::Record(ir_record) = ir_typedef {
                            return PathResolveResult::FunctionRef(ir_record.constructor);
                        } else {
                            unreachable!()
                        }
                    }
                    Item::Variant(_, _, ir_typedef_id, index) => {
                        let ir_typedef = ir_program
                            .typedefs
                            .get(&ir_typedef_id)
                            .expect("Adt not found");
                        if let TypeDef::Adt(ir_adt) = ir_typedef {
                            return PathResolveResult::FunctionRef(
                                ir_adt.variants[index].constructor,
                            );
                        } else {
                            unreachable!()
                        }
                    }
                    _ => {}
                }
            }
        }
        None => {
            let subs: Vec<_> = path.split(".").collect();
            let first = &subs[0];
            if let Some((named_ref, level)) = environment.get_ref(first) {
                let ir_expr = lambda_helper.process_named_ref(named_ref.clone(), level);
                let mut ir_expr_id = add_expr(ir_expr, id, ir_program, program);
                for sub in subs[1..].iter() {
                    match sub.parse::<usize>() {
                        Ok(index) => {
                            let ir_expr = IrExpr::TupleFieldAccess(index, ir_expr_id);
                            let next = add_expr(ir_expr, id, ir_program, program);
                            ir_expr_id = next;
                        }
                        Err(_) => {
                            let next = process_field_access(
                                id,
                                program,
                                module,
                                ir_program,
                                errors,
                                sub.to_string(),
                                ir_expr_id,
                                location_id,
                            );
                            ir_expr_id = next;
                        }
                    }
                }
                return PathResolveResult::VariableRef(ir_expr_id);
            }
        }
    }
    let err = ResolverError::UnknownFunction(path.to_string(), location_id);
    errors.push(err);
    let ir_expr = IrExpr::Tuple(vec![]);
    let ir_expr_id = add_expr(ir_expr, id, ir_program, program);
    return PathResolveResult::VariableRef(ir_expr_id);
}

fn add_expr(
    ir_expr: IrExpr,
    ast_id: ExprId,
    ir_program: &mut IrProgram,
    program: &Program,
) -> IrExprId {
    let expr_id = ir_program.get_expr_id();
    let location_id = program.get_expr_location(&ast_id);
    let expr_info = IrExprInfo::new(ir_expr, location_id);
    ir_program.add_expr(expr_id, expr_info);
    expr_id
}

fn process_field_access(
    id: ExprId,
    program: &Program,
    module: &Module,
    ir_program: &mut IrProgram,
    errors: &mut Vec<ResolverError>,
    name: String,
    ir_expr_id: IrExprId,
    location_id: LocationId,
) -> IrExprId {
    match module.imported_members.get(&name) {
        Some(members) => {
            let mut accesses = Vec::new();
            for member in members {
                match &member.member {
                    DataMember::Variant(..) => {}
                    DataMember::RecordField(record_field) => {
                        let access = FieldAccessInfo {
                            record_id: record_field.ir_typedef_id,
                            index: record_field.index,
                        };
                        accesses.push(access);
                    }
                }
            }
            let ir_expr = IrExpr::FieldAccess(accesses, ir_expr_id);
            return add_expr(ir_expr, id, ir_program, program);
        }
        None => {
            let err = ResolverError::UnknownFieldName(name.clone(), location_id);
            errors.push(err);
            let ir_expr = IrExpr::Tuple(vec![]);
            return add_expr(ir_expr, id, ir_program, program);
        }
    }
}

pub fn process_expr(
    id: ExprId,
    program: &Program,
    module: &Module,
    environment: &mut Environment,
    ir_program: &mut IrProgram,
    errors: &mut Vec<ResolverError>,
    lambda_helper: LambdaHelper,
) -> IrExprId {
    let expr = program.get_expr(&id);
    let location_id = program.get_expr_location(&id);
    //println!("Processing expr {} {}", id, expr);
    match expr {
        Expr::Lambda(args, lambda_body) => {
            let ir_lambda_id = ir_program.get_function_id();
            let mut arg_names = BTreeSet::new();
            let mut conflicting_names: BTreeSet<String> = BTreeSet::new();
            let mut environment = Environment::child(environment);
            for (index, arg) in args.iter().enumerate() {
                if !arg_names.insert(arg.0.clone()) {
                    conflicting_names.insert(arg.0.clone());
                }
                environment.add_arg(arg.0.clone(), ir_lambda_id, index);
            }
            if !conflicting_names.is_empty() {
                let err = ResolverError::LambdaArgumentConflict(
                    conflicting_names.into_iter().collect(),
                    location_id.clone(),
                );
                errors.push(err);
            }

            let local_lambda_helper = LambdaHelper::new(
                environment.level(),
                lambda_helper.host_function_name(),
                lambda_helper.clone_counter(),
                ir_lambda_id,
                lambda_helper.host_function(),
                Some(lambda_helper),
            );

            let ir_lambda_body = process_expr(
                *lambda_body,
                program,
                module,
                &mut environment,
                ir_program,
                errors,
                local_lambda_helper.clone(),
            );

            let lambda_info = LambdaInfo {
                body: ir_lambda_body,
                host_info: local_lambda_helper.host_function_name(),
                host_function: local_lambda_helper.host_function(),
                index: local_lambda_helper.get_lambda_index(),
                location_id: location_id,
            };

            let captures = local_lambda_helper.captures();

            let ir_function = IrFunction {
                id: ir_lambda_id,
                arg_locations: args.iter().map(|arg| arg.1).collect(),
                implicit_arg_count: captures.len(),
                info: FunctionInfo::Lambda(lambda_info),
            };
            ir_program.add_function(ir_lambda_id, ir_function);

            let captured_lambda_args: Vec<_> = captures
                .into_iter()
                .map(|expr| add_expr(expr, id, ir_program, program))
                .collect();
            let ir_expr = IrExpr::StaticFunctionCall(ir_lambda_id, captured_lambda_args);
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
                        lambda_helper.clone(),
                    )
                })
                .collect();
            let id_expr = program.get_expr(id_expr_id);
            if let Expr::Path(path) = id_expr {
                match resolve_item_path(
                    path,
                    module,
                    environment,
                    lambda_helper,
                    program,
                    ir_program,
                    id,
                    errors,
                    location_id,
                ) {
                    PathResolveResult::FunctionRef(n) => {
                        let ir_expr = IrExpr::StaticFunctionCall(n, ir_args);
                        return add_expr(ir_expr, id, ir_program, program);
                    }
                    PathResolveResult::VariableRef(ir_id_expr_id) => {
                        let ir_expr = IrExpr::DynamicFunctionCall(ir_id_expr_id, ir_args);
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
                        match resolve_item_path(
                            &path,
                            module,
                            environment,
                            lambda_helper.clone(),
                            program,
                            ir_program,
                            id,
                            errors,
                            location_id,
                        ) {
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
                        lambda_helper.clone(),
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
                lambda_helper.clone(),
            );
            let ir_true_branch = process_expr(
                *true_branch,
                program,
                module,
                environment,
                ir_program,
                errors,
                lambda_helper.clone(),
            );
            let ir_false_branch = process_expr(
                *false_branch,
                program,
                module,
                environment,
                ir_program,
                errors,
                lambda_helper.clone(),
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
                        lambda_helper.clone(),
                    )
                })
                .collect();
            let ir_expr = IrExpr::Tuple(ir_items);
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::Path(path) => {
            match resolve_item_path(
                path,
                module,
                environment,
                lambda_helper,
                program,
                ir_program,
                id,
                errors,
                location_id,
            ) {
                PathResolveResult::FunctionRef(n) => {
                    let ir_expr = IrExpr::StaticFunctionCall(n, vec![]);
                    return add_expr(ir_expr, id, ir_program, program);
                }
                PathResolveResult::VariableRef(ir_expr_id) => {
                    return ir_expr_id;
                }
            }
        }
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
                        lambda_helper.clone(),
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
            return process_field_access(
                id,
                program,
                module,
                ir_program,
                errors,
                name.to_string(),
                ir_expr_id,
                location_id,
            );
        }
        Expr::TupleFieldAccess(index, expr_id) => {
            let ir_expr_id = process_expr(
                *expr_id,
                program,
                module,
                environment,
                ir_program,
                errors,
                lambda_helper,
            );
            let ir_expr = IrExpr::TupleFieldAccess(*index, ir_expr_id);
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::Formatter(fmt, items) => {
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
                        lambda_helper.clone(),
                    )
                })
                .collect();
            let ir_expr = IrExpr::Formatter(fmt.clone(), ir_items);
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::Case(_) => unimplemented!(),
    }
}
