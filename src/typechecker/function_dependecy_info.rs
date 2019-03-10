use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::function::FunctionId;
use crate::ir::program::Program;
use crate::typechecker::collector::Collector;
use std::collections::BTreeSet;

pub struct FunctionDependencyInfo {
    pub function_deps: BTreeSet<FunctionId>,
}

impl FunctionDependencyInfo {
    pub fn new() -> FunctionDependencyInfo {
        FunctionDependencyInfo {
            function_deps: BTreeSet::new(),
        }
    }
}

pub struct FunctionInfoCollector<'a> {
    function_type_info: &'a mut FunctionDependencyInfo,
}

impl<'a> FunctionInfoCollector<'a> {
    pub fn new(function_type_info: &'a mut FunctionDependencyInfo) -> FunctionInfoCollector<'a> {
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
