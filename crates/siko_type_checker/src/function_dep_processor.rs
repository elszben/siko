use crate::common::FunctionTypeInfo;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_ir::walker::walk_expr;
use siko_ir::walker::Visitor;
#[allow(unused)]
use siko_util::format_list;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::dependency_processor::DependencyCollector;
use crate::dependency_processor::DependencyGroup;
use crate::dependency_processor::DependencyProcessor;

struct FunctionDependencyCollector<'a> {
    program: &'a Program,
    used_functions: BTreeSet<FunctionId>,
}

impl<'a> FunctionDependencyCollector<'a> {
    fn new(program: &'a Program) -> FunctionDependencyCollector<'a> {
        FunctionDependencyCollector {
            program: program,
            used_functions: BTreeSet::new(),
        }
    }
}

impl<'a> Visitor for FunctionDependencyCollector<'a> {
    fn get_program(&self) -> &Program {
        &self.program
    }

    fn visit_expr(&mut self, _: ExprId, expr: &Expr) {
        match expr {
            Expr::StaticFunctionCall(id, _) => {
                self.used_functions.insert(*id);
            }
            _ => {}
        }
    }

    fn visit_pattern(&mut self, _: PatternId, _: &Pattern) {
        // do nothing
    }
}

pub struct FunctionDependencyProcessor<'a> {
    program: &'a Program,
    function_type_info_map: &'a BTreeMap<FunctionId, FunctionTypeInfo>,
}

impl<'a> FunctionDependencyProcessor<'a> {
    pub fn new(
        program: &'a Program,
        function_type_info_map: &'a BTreeMap<FunctionId, FunctionTypeInfo>,
    ) -> FunctionDependencyProcessor<'a> {
        FunctionDependencyProcessor {
            program: program,
            function_type_info_map: function_type_info_map,
        }
    }

    pub fn process_functions(&self, program: &Program) -> Vec<DependencyGroup<FunctionId>> {
        let mut functions = Vec::new();
        for (id, _) in &program.functions.items {
            if let Some(type_info) = self.function_type_info_map.get(id) {
                if let Some(_) = type_info.body {
                    functions.push(*id);
                }
            }
        }
        let dep_processor = DependencyProcessor::new(functions);
        let ordered_function_groups = dep_processor.process_items(self);

        ordered_function_groups
    }
}

impl<'a> DependencyCollector<FunctionId> for FunctionDependencyProcessor<'a> {
    fn collect(&self, function_id: FunctionId) -> Vec<FunctionId> {
        let type_info = self.function_type_info_map.get(&function_id).unwrap();
        let body = type_info.body.unwrap();
        let mut collector = FunctionDependencyCollector::new(self.program);
        walk_expr(&body, &mut collector);
        let deps: Vec<_> = collector.used_functions.into_iter().collect();
        //println!("{} deps {}", id, format_list(&deps[..]));
        let mut deps: BTreeSet<_> = deps
            .iter()
            .filter(|dep_id| {
                let dep_info = self
                    .function_type_info_map
                    .get(dep_id)
                    .expect("type info not found");
                !dep_info.typed
            })
            .map(|id| *id)
            .collect();
        let func_info = self.program.functions.get(&function_id);
        if let Some(host) = func_info.get_lambda_host() {
            deps.insert(host);
        }
        deps.into_iter().collect()
    }
}
