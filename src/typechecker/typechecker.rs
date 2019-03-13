use crate::error::Error;
use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::function::FunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::program::Program;
use crate::ir::types::TypeSignature;
use crate::ir::types::TypeSignatureId;
use crate::typechecker::collector::Collector;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::function_dependecy_info::FunctionDependencyInfo;
use crate::typechecker::function_dependecy_info::FunctionInfoCollector;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::type_processor::TypeProcessor;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

fn walker(program: &Program, id: &ExprId, collector: &mut Collector) {
    let expr = program.get_expr(id);
    //println!("TC: {}: Processing expr {}", id, expr);
    match expr {
        Expr::StaticFunctionCall(_, args) => {
            for arg in args {
                walker(program, arg, collector);
            }
        }
        Expr::LambdaFunction(lambda_id, captures) => {
            let lambda = program.get_function(lambda_id);
            for captured in captures {
                walker(program, captured, collector);
            }
            collector.process(program, expr, *id);
            if let FunctionInfo::Lambda(info) = &lambda.info {
                walker(program, &info.body, collector);
            } else {
                unreachable!()
            }
            return;
        }
        Expr::DynamicFunctionCall(id, args) => {
            walker(program, id, collector);
            for arg in args {
                walker(program, arg, collector);
            }
        }
        Expr::If(cond, true_branch, false_branch) => {
            walker(program, cond, collector);
            walker(program, true_branch, collector);
            walker(program, false_branch, collector);
        }
        Expr::Tuple(items) => {
            for item in items {
                walker(program, item, collector)
            }
        }
        Expr::IntegerLiteral(_) => {}
        Expr::FloatLiteral(_) => {}
        Expr::BoolLiteral(_) => {}
        Expr::StringLiteral(_) => {}
        Expr::Do(items) => {
            for item in items {
                walker(program, item, collector)
            }
        }
        Expr::Bind(_, expr) => walker(program, expr, collector),
        Expr::ArgRef(_) => {}
        Expr::ExprValue(_) => {}
        Expr::LambdaCapturedArgRef(_) => {}
    }
    collector.process(program, &expr, *id);
}

pub struct Typechecker {
    function_info_map: BTreeMap<FunctionId, FunctionDependencyInfo>,
    function_type_map: BTreeMap<FunctionId, TypeVariable>,
    type_store: TypeStore,
}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {
            function_info_map: BTreeMap::new(),
            function_type_map: BTreeMap::new(),
            type_store: TypeStore::new(),
        }
    }

    fn check_untyped_function(
        &mut self,
        id: FunctionId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        let function = program.get_function(&id);
        // println!("Checking untyped {},{}", id, function.info);
        let mut args = Vec::new();
        for _ in 0..function.arg_count {
            let ty = self.type_store.get_unique_type_arg_type();
            let var = self.type_store.add_var(ty);
            args.push(var);
        }
        let body = function.info.body();
        let mut type_processor =
            TypeProcessor::new(&mut self.type_store, &self.function_type_map, id, args);
        walker(program, &body, &mut type_processor);
        type_processor.check_constraints(program, errors);
        // type_processor.dump_types(program);
        let function_type = type_processor.get_function_type(&body);
        /*println!(
            "Type of {},{}: {}",
            id,
            function.info,
            self.type_store.get_resolved_type_string(&function_type)
        );*/
        self.function_type_map.insert(id, function_type);
    }

    fn check_typed_function(
        &mut self,
        id: FunctionId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        let function = program.get_function(&id);
        //println!("Checking typed {},{}", id, function.info);
        let function_type_var = self
            .function_type_map
            .get(&id)
            .expect("Function type not found");
        let mut args = Vec::new();
        let function_type = self.type_store.get_type(function_type_var);
        let function_type = self.type_store.clone_type(&function_type);
        let mut expected_result_var = *function_type_var;
        match function_type {
            Type::Function(function_type) => {
                expected_result_var = function_type.get_return_type();
                args.extend(function_type.get_arg_types());
            }
            _ => {}
        }
        let body = function.info.body();
        let mut type_processor =
            TypeProcessor::new(&mut self.type_store, &self.function_type_map, id, args);
        walker(program, &body, &mut type_processor);
        type_processor.check_constraints(program, errors);
        //type_processor.dump_types(program);
        let inferred_function_type_var = type_processor.get_function_type(&body);
        /* println!(
            "Type of {},{}: {}",
            id,
            function.info,
            self.type_store
                .get_resolved_type_string(&inferred_function_type_var)
        );*/
        let inferred_function_type = self.type_store.get_type(&inferred_function_type_var);
        let inferred_result_var: TypeVariable = match inferred_function_type {
            Type::Function(inferred_function_type) => inferred_function_type.get_return_type(),
            _ => inferred_function_type_var,
        };
        let mut unified_variables = false;

        if !self.type_store.unify_vars(
            &expected_result_var,
            &inferred_result_var,
            &mut unified_variables,
        ) {
            let ast_id = program.get_ast_expr_id(&body);
            let body_type = self
                .type_store
                .get_resolved_type_string(&inferred_result_var);
            let expected_result_type = self
                .type_store
                .get_resolved_type_string(&expected_result_var);
            let err = TypecheckError::TypeMismatch(*ast_id, expected_result_type, body_type);
            errors.push(err);
        }
    }

    fn check_function_deps(
        &self,
        mut untyped_functions: BTreeSet<FunctionId>,
        errors: &mut Vec<TypecheckError>,
        program: &Program,
    ) -> Vec<FunctionId> {
        let mut untyped_check_order = Vec::new();
        while !untyped_functions.is_empty() {
            let mut processed = Vec::new();
            for id in &untyped_functions {
                let info = self
                    .function_info_map
                    .get(id)
                    .expect("Function info not found");
                let mut dep_is_untyped = false;
                for dep in &info.function_deps {
                    if untyped_functions.contains(dep) {
                        dep_is_untyped = true;
                        break;
                    }
                }
                if dep_is_untyped {
                    continue;
                } else {
                    untyped_check_order.push(*id);
                    processed.push(*id);
                }
            }
            if processed.is_empty() {
                for id in &untyped_functions {
                    let f = program.get_function(id);
                    println!("Untyped function: {}", f.info);
                }
                let err = TypecheckError::FunctionTypeDependencyLoop;
                errors.push(err);
                break;
            } else {
                for id in processed {
                    untyped_functions.remove(&id);
                }
            }
        }
        untyped_check_order
    }

    fn process_type_signature(
        &mut self,
        type_signature_id: &TypeSignatureId,
        program: &Program,
        arg_map: &mut BTreeMap<usize, TypeVariable>,
    ) -> TypeVariable {
        let type_signature = program.get_type_signature(type_signature_id);
        match type_signature {
            TypeSignature::Bool => {
                let ty = Type::Bool;
                return self.type_store.add_var(ty);
            }
            TypeSignature::Int => {
                let ty = Type::Int;
                return self.type_store.add_var(ty);
            }
            TypeSignature::String => {
                let ty = Type::String;
                return self.type_store.add_var(ty);
            }
            TypeSignature::Nothing => {
                let ty = Type::Nothing;
                return self.type_store.add_var(ty);
            }
            TypeSignature::Tuple(items) => {
                let items: Vec<_> = items
                    .iter()
                    .map(|i| self.process_type_signature(i, program, arg_map))
                    .collect();
                let ty = Type::Tuple(items);
                return self.type_store.add_var(ty);
            }
            TypeSignature::Function(items) => {
                let items: Vec<_> = items
                    .iter()
                    .map(|i| self.process_type_signature(i, program, arg_map))
                    .collect();
                let ty = Type::Function(FunctionType::new(items));
                return self.type_store.add_var(ty);
            }
            TypeSignature::TypeArgument(index) => {
                let var = arg_map.entry(*index).or_insert_with(|| {
                    let arg = self.type_store.get_unique_type_arg();
                    let ty = Type::TypeArgument {
                        index: arg,
                        user_defined: true,
                    };
                    self.type_store.add_var(ty)
                });
                *var
            }
        }
    }

    fn add_type_signature(
        &mut self,
        type_signature_id: TypeSignatureId,
        function_id: FunctionId,
        program: &Program,
    ) {
        let mut arg_map = BTreeMap::new();
        let var = self.process_type_signature(&type_signature_id, program, &mut arg_map);
        /*println!(
            "Registering function {} with type {}",
            function_id,
            self.type_store.get_resolved_type(&var)
        );*/
        self.function_type_map.insert(function_id, var);
    }

    pub fn check(&mut self, program: &Program) -> Result<(), Error> {
        let mut errors = Vec::new();
        let mut untyped_functions = BTreeSet::new();
        let mut typed_functions = BTreeSet::new();
        let mut extern_count = 0;
        //println!("All function count {}", program.functions.len());
        for (id, function) in &program.functions {
            let mut function_info = FunctionDependencyInfo::new();
            let mut function_info_collector = FunctionInfoCollector::new(&mut function_info);
            match &function.info {
                FunctionInfo::Lambda(_) => {}
                FunctionInfo::NamedFunction(i) => {
                    let untyped = match i.type_signature {
                        Some(type_signature) => {
                            self.add_type_signature(type_signature, *id, program);
                            false
                        }
                        None => true,
                    };
                    if untyped {
                        untyped_functions.insert(*id);
                    }
                    if let Some(body) = i.body {
                        walker(program, &body, &mut function_info_collector);
                        if !untyped {
                            typed_functions.insert(*id);
                        }
                    } else {
                        extern_count += 1;
                        if untyped {
                            let err = TypecheckError::UntypedExternFunction(
                                i.name.clone(),
                                i.ast_function_id,
                            );
                            errors.push(err)
                        }
                    }
                }
            }
            self.function_info_map.insert(*id, function_info);
        }

        let untyped_check_order = self.check_function_deps(untyped_functions, &mut errors, program);

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }
        /*
                println!(
                    "Typed: {}, untyped: {}, extern: {}",
                    typed_functions.len(),
                    untyped_check_order.len(),
                    extern_count
                );
        */
        for function_id in typed_functions {
            self.check_typed_function(function_id, program, &mut errors);
        }

        for function_id in untyped_check_order {
            self.check_untyped_function(function_id, program, &mut errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::typecheck_err(errors))
        }
    }
}
