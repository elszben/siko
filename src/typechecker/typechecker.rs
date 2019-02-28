use crate::error::Error;
use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::function::FunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::program::Program;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;

use std::collections::BTreeMap;
use std::collections::BTreeSet;

struct FunctionTypeInfo {
    function_deps: BTreeSet<FunctionId>,
}

impl FunctionTypeInfo {
    fn new() -> FunctionTypeInfo {
        FunctionTypeInfo {
            function_deps: BTreeSet::new(),
        }
    }
}

struct FunctionInfoCollector<'a> {
    function_type_info: &'a mut FunctionTypeInfo,
}

impl<'a> FunctionInfoCollector<'a> {
    fn new(function_type_info: &'a mut FunctionTypeInfo) -> FunctionInfoCollector<'a> {
        FunctionInfoCollector {
            function_type_info: function_type_info,
        }
    }
}

impl<'a> Collector for FunctionInfoCollector<'a> {
    fn process(&mut self, program: &Program, expr: &Expr, id: ExprId) {
        match expr {
            Expr::StaticFunctionCall(func_id, _) => {
                self.function_type_info.function_deps.insert(*func_id);
            }
            Expr::LambdaFunction(func_id, _) => {
                self.function_type_info.function_deps.insert(*func_id);
            }
            _ => {}
        }
    }
}

struct TypeProcessor {
    type_store: TypeStore,
    type_vars: BTreeMap<ExprId, TypeVariable>,
}

impl TypeProcessor {
    fn new() -> TypeProcessor {
        TypeProcessor {
            type_store: TypeStore::new(),
            type_vars: BTreeMap::new(),
        }
    }

    fn get_type_var_for_expr(&self, id: &ExprId) -> TypeVariable {
        self.type_vars
            .get(id)
            .expect("Sub expr type var not found")
            .clone()
    }

    fn check_constraints(&mut self, program: &Program, errors: &mut Vec<TypecheckError>) {
        for (id, ty_var) in &self.type_vars {
            let expr = program.get_expr(id);
            match expr {
                Expr::IntegerLiteral(_) => {}
                Expr::BoolLiteral(_) => {}
                Expr::If(cond, true_branch, false_branch) => {
                    let cond_var = self.get_type_var_for_expr(cond);
                    let true_var = self.get_type_var_for_expr(true_branch);
                    let false_var = self.get_type_var_for_expr(false_branch);
                    let cond_ty = self.type_store.get_type(cond_var);
                    if cond_ty != Type::Bool {
                        let var = self.type_store.add_var(Type::Bool);
                        if !self.type_store.unify_vars(var, cond_var) {
                            let ast_cond_id = program.get_ast_expr_id(cond);
                            let cond_ty = self.type_store.get_resolved_type(cond_var);
                            let cond_ty = format!("{}", cond_ty);
                            let err = TypecheckError::IfCondition(*ast_cond_id, cond_ty);
                            errors.push(err);
                        }
                    }
                    if !self.type_store.unify_vars(true_var, false_var) {
                        let ast_if_id = program.get_ast_expr_id(id);
                        let true_type = self.type_store.get_resolved_type(true_var);
                        let false_type = self.type_store.get_resolved_type(false_var);
                        let true_type = format!("{}", true_type);
                        let false_type = format!("{}", false_type);
                        let err =
                            TypecheckError::IfBranchMismatch(*ast_if_id, true_type, false_type);
                        errors.push(err);
                    }
                }
                _ => panic!("Check of expr {} is not implemented", expr),
            }
        }
    }
}

impl<'a> Collector for TypeProcessor {
    fn process(&mut self, program: &Program, expr: &Expr, id: ExprId) {
        match expr {
            Expr::IntegerLiteral(_) => {
                let ty = Type::Int;
                let var = self.type_store.add_var(ty.clone());
                self.type_vars.insert(id, var);
            }
            Expr::BoolLiteral(_) => {
                let ty = Type::Bool;
                let var = self.type_store.add_var(ty.clone());
                self.type_vars.insert(id, var);
            }
            Expr::If(cond, true_branch, false_branch) => {
                let true_var = self.get_type_var_for_expr(true_branch);
                self.type_vars.insert(id, true_var);
            }
            _ => {}
        }
    }
}

trait Collector {
    fn process(&mut self, program: &Program, expr: &Expr, id: ExprId) {}
}

fn walker(program: &Program, id: &ExprId, collector: &mut Collector) {
    let expr = program.get_expr(id);

    match expr {
        Expr::StaticFunctionCall(_, args) => {
            for arg in args {
                walker(program, arg, collector);
            }
        }
        Expr::LambdaFunction(_, args) => {
            for arg in args {
                walker(program, arg, collector);
            }
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
        Expr::VariableRef(_) => {}
    }
    collector.process(program, &expr, *id);
}

pub struct Typechecker {
    function_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {
            function_info_map: BTreeMap::new(),
        }
    }

    fn check_untyped_function(
        &self,
        id: FunctionId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        println!("Checking untyped {}", id);
        let function = program.get_function(&id);
        let body = function.info.body();
        let mut type_processor = TypeProcessor::new();
        walker(program, &body, &mut type_processor);
        type_processor.check_constraints(program, errors);
    }

    fn check_function_deps(
        &self,
        mut untyped_functions: BTreeSet<FunctionId>,
        errors: &mut Vec<TypecheckError>,
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

    pub fn check(&mut self, program: &Program) -> Result<(), Error> {
        let mut errors = Vec::new();
        let mut untyped_functions = BTreeSet::new();
        let mut typed_functions = BTreeSet::new();
        for (id, function) in &program.functions {
            let mut function_info = FunctionTypeInfo::new();
            let mut function_info_collector = FunctionInfoCollector::new(&mut function_info);
            match &function.info {
                FunctionInfo::Lambda(e) => {
                    walker(program, &e, &mut function_info_collector);
                    untyped_functions.insert(*id);
                }
                FunctionInfo::NamedFunction(i) => {
                    let untyped = i.type_signature.is_none();
                    if untyped {
                        untyped_functions.insert(*id);
                    }
                    if let Some(body) = i.body {
                        walker(program, &body, &mut function_info_collector);
                        if !untyped {
                            typed_functions.insert(*id);
                        }
                    } else {
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

        let untyped_check_order = self.check_function_deps(untyped_functions, &mut errors);

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        for function_id in untyped_check_order {
            self.check_untyped_function(function_id, program, &mut errors);
        }

        for function_id in typed_functions {
            println!("Checking typed {}", function_id);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::typecheck_err(errors))
        }
    }
}
