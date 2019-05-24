use crate::constants::BuiltinOperator;
use crate::constants::PRELUDE_NAME;
use crate::ir::expr::Case as IrCase;
use crate::ir::expr::Expr as IrExpr;
use crate::ir::expr::ExprId as IrExprId;
use crate::ir::expr::ExprInfo as IrExprInfo;
use crate::ir::expr::FieldAccessInfo;
use crate::ir::expr::RecordFieldValueExpr;
use crate::ir::function::Function as IrFunction;
use crate::ir::function::FunctionId as IrFunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::function::LambdaInfo;
use crate::ir::pattern::Pattern as IrPattern;
use crate::ir::pattern::PatternId as IrPatternId;
use crate::ir::pattern::PatternInfo as IrPatternInfo;
use crate::ir::program::Program as IrProgram;
use crate::ir::types::TypeDef;
use crate::ir::types::TypeDefId;
use crate::location_info::item::LocationId;
use crate::name_resolution::environment::Environment;
use crate::name_resolution::error::ResolverError;
use crate::name_resolution::item::DataMember;
use crate::name_resolution::item::Item;
use crate::name_resolution::lambda_helper::LambdaHelper;
use crate::name_resolution::module::Module;
use crate::syntax::expr::Expr;
use crate::syntax::expr::ExprId;
use crate::syntax::pattern::Pattern;
use crate::syntax::pattern::PatternId;
use crate::syntax::program::Program;
use std::collections::BTreeMap;
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
    if let Some((named_ref, level)) = environment.get_ref(path) {
        let ir_expr = lambda_helper.process_named_ref(named_ref.clone(), level);
        let ir_expr_id = add_expr(ir_expr, id, ir_program, program);
        return PathResolveResult::VariableRef(ir_expr_id);
    }
    if let Some(items) = module.imported_items.get(path) {
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
                        return PathResolveResult::FunctionRef(ir_adt.variants[index].constructor);
                    } else {
                        unreachable!()
                    }
                }
                _ => {}
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

fn resolve_pattern_type_constructor(
    name: &String,
    ir_program: &mut IrProgram,
    module: &Module,
    errors: &mut Vec<ResolverError>,
    location_id: LocationId,
    ids: Vec<IrPatternId>,
    irrefutable: bool,
) -> IrPattern {
    if let Some(items) = module.imported_items.get(name) {
        if items.len() > 1 {
            let err = ResolverError::AmbiguousName(name.to_string(), location_id);
            errors.push(err);
            return IrPattern::Wildcard;
        } else {
            let item = &items[0];
            match item.item {
                Item::Function(_, _) => unreachable!(),
                Item::Record(_, ir_typedef_id) => {
                    let ir_typedef = ir_program
                        .typedefs
                        .get(&ir_typedef_id)
                        .expect("Record not found");
                    if let TypeDef::Record(..) = ir_typedef {
                        return IrPattern::Record(ir_typedef_id, ids);
                    } else {
                        unreachable!()
                    }
                }
                Item::Variant(_, _, ir_typedef_id, index) => {
                    if irrefutable {
                        let err = ResolverError::NotIrrefutablePattern(location_id);
                        errors.push(err);
                        return IrPattern::Wildcard;
                    } else {
                        let ir_typedef = ir_program
                            .typedefs
                            .get(&ir_typedef_id)
                            .expect("Adt not found");
                        if let TypeDef::Adt(..) = ir_typedef {
                            return IrPattern::Variant(ir_typedef_id, index, ids);
                        } else {
                            unreachable!()
                        }
                    }
                }
                _ => {}
            }
        }
    };
    let err = ResolverError::UnknownTypeName(name.to_string(), location_id);
    errors.push(err);
    return IrPattern::Wildcard;
}

fn resolve_record_init_type(
    name: &String,
    ir_program: &mut IrProgram,
    module: &Module,
    errors: &mut Vec<ResolverError>,
    location_id: LocationId,
) -> Option<TypeDefId> {
    if let Some(items) = module.imported_items.get(name) {
        if items.len() > 1 {
            let err = ResolverError::AmbiguousName(name.to_string(), location_id);
            errors.push(err);
            return None;
        } else {
            let item = &items[0];
            match item.item {
                Item::Function(_, _) => unreachable!(),
                Item::Record(_, ir_typedef_id) => {
                    let ir_typedef = ir_program
                        .typedefs
                        .get(&ir_typedef_id)
                        .expect("Record not found");
                    if let TypeDef::Record(..) = ir_typedef {
                        return Some(ir_typedef_id);
                    } else {
                        unreachable!()
                    }
                }
                Item::Variant(..) => {
                    let err = ResolverError::NotRecordType(name.clone(), location_id);
                    errors.push(err);
                    return None;
                }
                _ => {}
            }
        }
    };
    let err = ResolverError::UnknownTypeName(name.to_string(), location_id);
    errors.push(err);
    return None;
}

fn process_pattern(
    case_expr_id: IrExprId,
    pattern_id: PatternId,
    program: &Program,
    ir_program: &mut IrProgram,
    module: &Module,
    environment: &mut Environment,
    bindings: &mut BTreeMap<String, Vec<LocationId>>,
    errors: &mut Vec<ResolverError>,
    lambda_helper: LambdaHelper,
    irrefutable: bool,
) -> IrPatternId {
    let ir_pattern_id = ir_program.get_pattern_id();
    let (pattern, location) = program
        .patterns
        .get(&pattern_id)
        .expect("Pattern not found");
    let ir_pattern = match pattern {
        Pattern::Binding(name) => {
            let locations = bindings.entry(name.clone()).or_insert_with(|| Vec::new());
            locations.push(*location);
            environment.add_expr_value(name.clone(), case_expr_id, ir_pattern_id);
            IrPattern::Binding(name.clone())
        }
        Pattern::Tuple(patterns) => {
            let ids: Vec<_> = patterns
                .iter()
                .map(|id| {
                    process_pattern(
                        case_expr_id,
                        *id,
                        program,
                        ir_program,
                        module,
                        environment,
                        bindings,
                        errors,
                        lambda_helper.clone(),
                        irrefutable,
                    )
                })
                .collect();
            IrPattern::Tuple(ids)
        }
        Pattern::Constructor(name, patterns) => {
            let ids: Vec<_> = patterns
                .iter()
                .map(|id| {
                    process_pattern(
                        case_expr_id,
                        *id,
                        program,
                        ir_program,
                        module,
                        environment,
                        bindings,
                        errors,
                        lambda_helper.clone(),
                        irrefutable,
                    )
                })
                .collect();
            resolve_pattern_type_constructor(
                name,
                ir_program,
                module,
                errors,
                *location,
                ids,
                irrefutable,
            )
        }
        Pattern::Guarded(pattern_id, guard_expr_id) => {
            let ir_pattern_id = process_pattern(
                case_expr_id,
                *pattern_id,
                program,
                ir_program,
                module,
                environment,
                bindings,
                errors,
                lambda_helper.clone(),
                irrefutable,
            );
            let ir_guard_expr_id = process_expr(
                *guard_expr_id,
                program,
                module,
                environment,
                ir_program,
                errors,
                lambda_helper.clone(),
            );
            IrPattern::Guarded(ir_pattern_id, ir_guard_expr_id)
        }
        Pattern::Wildcard => IrPattern::Wildcard,
        Pattern::IntegerLiteral(v) => {
            if irrefutable {
                let err = ResolverError::NotIrrefutablePattern(*location);
                errors.push(err);
                IrPattern::Wildcard
            } else {
                IrPattern::IntegerLiteral(*v)
            }
        }
        Pattern::FloatLiteral(v) => {
            if irrefutable {
                let err = ResolverError::NotIrrefutablePattern(*location);
                errors.push(err);
                IrPattern::Wildcard
            } else {
                IrPattern::FloatLiteral(*v)
            }
        }
        Pattern::StringLiteral(v) => {
            if irrefutable {
                let err = ResolverError::NotIrrefutablePattern(*location);
                errors.push(err);
                IrPattern::Wildcard
            } else {
                IrPattern::StringLiteral(v.clone())
            }
        }
        Pattern::BoolLiteral(v) => {
            if irrefutable {
                let err = ResolverError::NotIrrefutablePattern(*location);
                errors.push(err);
                IrPattern::Wildcard
            } else {
                IrPattern::BoolLiteral(*v)
            }
        }
    };
    let ir_pattern_info = IrPatternInfo {
        pattern: ir_pattern,
        location_id: *location,
    };
    ir_program.add_pattern(ir_pattern_id, ir_pattern_info);
    ir_pattern_id
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
        Expr::Bind(pattern_id, expr_id) => {
            let ir_expr_id = process_expr(
                *expr_id,
                program,
                module,
                environment,
                ir_program,
                errors,
                lambda_helper.clone(),
            );
            let mut bindings = BTreeMap::new();
            let ir_pattern_id = process_pattern(
                ir_expr_id,
                *pattern_id,
                program,
                ir_program,
                module,
                environment,
                &mut bindings,
                errors,
                lambda_helper.clone(),
                true,
            );
            let ir_expr = IrExpr::Bind(ir_pattern_id, ir_expr_id);
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
        Expr::CaseOf(body_id, cases) => {
            let ir_body_id = process_expr(
                *body_id,
                program,
                module,
                environment,
                ir_program,
                errors,
                lambda_helper.clone(),
            );
            let mut ir_cases = Vec::new();
            for case in cases {
                let mut case_environment = Environment::child(environment);
                let mut bindings = BTreeMap::new();
                let pattern_id = process_pattern(
                    ir_body_id,
                    case.pattern_id,
                    program,
                    ir_program,
                    module,
                    &mut case_environment,
                    &mut bindings,
                    errors,
                    lambda_helper.clone(),
                    false,
                );
                let ir_case_body_id = process_expr(
                    case.body,
                    program,
                    module,
                    &mut case_environment,
                    ir_program,
                    errors,
                    lambda_helper.clone(),
                );
                let ir_case = IrCase {
                    pattern_id: pattern_id,
                    body: ir_case_body_id,
                };
                ir_cases.push(ir_case);
            }
            let ir_expr = IrExpr::CaseOf(ir_body_id, ir_cases);
            return add_expr(ir_expr, id, ir_program, program);
        }
        Expr::RecordInitialization(name, items) => {
            if let Some(ir_type_id) =
                resolve_record_init_type(name, ir_program, module, errors, location_id)
            {
                let record = ir_program.get_record(&ir_type_id).clone();
                let mut unused_fields = BTreeSet::new();
                let mut initialized_twice = BTreeSet::new();
                for f in &record.fields {
                    unused_fields.insert(f.name.clone());
                }
                let ir_items: Vec<_> = items
                    .iter()
                    .map(|i| {
                        let mut field_index = None;
                        for (index,f) in record.fields.iter().enumerate() {
                            if f.name == i.field_name {
                                field_index = Some(index);
                                if !unused_fields.remove(&f.name) {
                                    initialized_twice.insert(f.name.clone());
                                }
                            }
                        }
                        let field_index = match field_index {
                            None => { let err = ResolverError::NoSuchField(
                                record.name.clone(),
                                i.field_name.clone(),
                                i.location_id,
                            );
                            errors.push(err);
                            0
                            }
                            Some(i) => i
                        };
                        let ir_body_id = process_expr(
                            i.body,
                            program,
                            module,
                            environment,
                            ir_program,
                            errors,
                            lambda_helper.clone(),
                        );
                        let value_expr = RecordFieldValueExpr {
                            expr_id : ir_body_id,
                            index: field_index
                        };
                        value_expr
                    })
                    .collect();
                if !unused_fields.is_empty() {
                    let err = ResolverError::MissingFields(
                        unused_fields.into_iter().collect(),
                        location_id,
                    );
                    errors.push(err);
                }
                if !initialized_twice.is_empty() {
                    let err = ResolverError::FieldsInitializedMultipleTimes(
                        initialized_twice.into_iter().collect(),
                        location_id,
                    );
                    errors.push(err);
                }
                let ir_expr = IrExpr::RecordInitialization(ir_type_id, ir_items);
                return add_expr(ir_expr, id, ir_program, program);
            } else {
                let ir_expr = IrExpr::Tuple(vec![]);
                return add_expr(ir_expr, id, ir_program, program);
            }
        }
        Expr::RecordUpdate(name, items) => unimplemented!(),
    }
}
