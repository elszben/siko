use crate::error::Error;
use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::function::FunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::program::Program;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::type_variable::TypeVariable;

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

struct TypeVariableCollector {}

impl<'a> Collector for TypeVariableCollector {
    fn process(&mut self, program: &Program, expr: &Expr, id: ExprId) {

    }
}

trait Collector {
    fn process(&mut self, program: &Program, expr: &Expr, id: ExprId) {}
}

fn walker(program: &Program, id: &ExprId, collector: &mut Collector) {
    let expr = program.get_expr(id);
    collector.process(program, &expr, *id);
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
