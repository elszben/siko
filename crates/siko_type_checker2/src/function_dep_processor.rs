use crate::common::FunctionTypeInfo;
use crate::common::FunctionTypeInfoStore;
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
    function_type_info_store: &'a FunctionTypeInfoStore,
}

impl<'a> FunctionDependencyProcessor<'a> {
    pub fn new(
        program: &'a Program,
        function_type_info_store: &'a FunctionTypeInfoStore,
    ) -> FunctionDependencyProcessor<'a> {
        FunctionDependencyProcessor {
            program: program,
            function_type_info_store: function_type_info_store,
        }
    }

    pub fn process_functions(&self) -> Vec<DependencyGroup<FunctionId>> {
        let mut functions = Vec::new();
        for (id, info) in &self.program.functions.items {
            // hack
            let displayed_name = format!("{}", info.info);
            if displayed_name != "Main/main" {
                continue;
            }
            let type_info = self.function_type_info_store.get(id);
            if let Some(_) = type_info.body {
                functions.push(*id);
            }
        }

        let dep_processor = DependencyProcessor::new(functions);
        let ordered_function_groups = dep_processor.process_items(self);

        ordered_function_groups
    }
}

impl<'a> DependencyCollector<FunctionId> for FunctionDependencyProcessor<'a> {
    fn collect(&self, function_id: FunctionId) -> Vec<FunctionId> {
        let type_info = self.function_type_info_store.get(&function_id);
        let body = type_info.body.unwrap();
        let mut collector = FunctionDependencyCollector::new(self.program);
        walk_expr(&body, &mut collector);
        let deps: Vec<_> = collector.used_functions.into_iter().collect();
        //println!("{} deps {}", id, format_list(&deps[..]));
        let mut deps: BTreeSet<_> = deps
            .iter()
            .filter(|dep_id| {
                let dep_info = self.function_type_info_store.get(dep_id);
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
