use crate::common::DependencyGroup;
use crate::common::FunctionTypeInfo;
use crate::type_store::TypeStore;
use crate::walker::walk_expr;
use crate::walker::Visitor;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
#[allow(unused)]
use siko_util::format_list;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

struct FunctionDependency {
    function_deps: BTreeSet<FunctionId>,
}

struct GroupTable {
    table: BTreeMap<FunctionId, usize>,
}

impl GroupTable {
    fn new() -> GroupTable {
        GroupTable {
            table: BTreeMap::new(),
        }
    }

    fn merge(&mut self, f1: FunctionId, f2: FunctionId) {
        let id1 = *self
            .table
            .get(&f1)
            .expect("Function in group table not found");
        let id2 = *self
            .table
            .get(&f2)
            .expect("Function in group table not found");
        for (_, group_id) in self.table.iter_mut() {
            if *group_id == id1 {
                *group_id = id2;
            }
        }
    }
}

struct DependencyCollector {
    used_functions: BTreeSet<FunctionId>,
}

impl DependencyCollector {
    fn new() -> DependencyCollector {
        DependencyCollector {
            used_functions: BTreeSet::new(),
        }
    }
}

impl Visitor for DependencyCollector {
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

pub struct FunctionDependencyProcessor {
    type_store: TypeStore,
    function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    function_deps: BTreeMap<FunctionId, FunctionDependency>,
}

impl FunctionDependencyProcessor {
    pub fn new(
        type_store: TypeStore,
        function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    ) -> FunctionDependencyProcessor {
        FunctionDependencyProcessor {
            type_store: type_store,
            function_type_info_map: function_type_info_map,
            function_deps: BTreeMap::new(),
        }
    }

    fn collect_deps(&mut self, program: &Program) {
        for (id, type_info) in &self.function_type_info_map {
            if let Some(body) = type_info.body {
                let mut collector = DependencyCollector::new();
                walk_expr(&body, program, &mut collector);
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
                let func_info = program.functions.get(id);
                if let Some(host) = func_info.get_lambda_host() {
                    deps.insert(host);
                }
                self.function_deps.insert(
                    *id,
                    FunctionDependency {
                        function_deps: deps,
                    },
                );
            }
        }
    }

    fn depends_on(
        &self,
        user: &FunctionId,
        used_function: &FunctionId,
        visited: &mut BTreeSet<FunctionId>,
    ) -> bool {
        if !visited.insert(*user) {
            return false;
        }
        let deps = self.function_deps.get(user).expect("dep info not found");
        if deps.function_deps.contains(used_function) {
            return true;
        } else {
            for dep in &deps.function_deps {
                if self.depends_on(dep, used_function, visited) {
                    return true;
                }
            }
        }
        false
    }

    pub fn process_functions(
        mut self,
        program: &Program,
    ) -> (
        TypeStore,
        BTreeMap<FunctionId, FunctionTypeInfo>,
        Vec<DependencyGroup>,
    ) {
        self.collect_deps(program);

        let mut group_table = GroupTable::new();

        for (index, (id, _)) in self.function_deps.iter().enumerate() {
            group_table.table.insert(*id, index);
        }

        for (id, d) in &self.function_deps {
            for dep in &d.function_deps {
                let mut visited = BTreeSet::new();
                if self.depends_on(dep, id, &mut visited) {
                    group_table.merge(*id, *dep);
                }
            }
        }

        let mut groups = BTreeMap::new();
        let mut unprocessed_groups = BTreeSet::new();
        for (id, group_id) in &group_table.table {
            let group = groups
                .entry(*group_id)
                .or_insert_with(|| DependencyGroup::new());
            group.functions.insert(*id);
            unprocessed_groups.insert(*group_id);
        }

        let mut processed_functions = BTreeSet::new();

        let mut ordered_groups = Vec::new();

        while !unprocessed_groups.is_empty() {
            let copied = unprocessed_groups.clone();
            let mut found = false;
            for group_id in &copied {
                let group = groups.get(group_id).expect("group not found");
                assert!(!group.functions.is_empty());
                let mut dep_missing = false;
                for function in &group.functions {
                    let deps = self.function_deps.get(function).expect("dep not found");
                    for dep in &deps.function_deps {
                        if !processed_functions.contains(dep) && !group.functions.contains(dep) {
                            dep_missing = true;
                        }
                    }
                }
                if !dep_missing {
                    //println!("Processing group {}", group_id);
                    ordered_groups.push(group.clone());
                    for function in &group.functions {
                        processed_functions.insert(function);
                    }
                    unprocessed_groups.remove(group_id);
                    found = true;
                    break;
                }
            }
            if !found {
                panic!("Cyclic dep groups");
            }
        }

        /*
        for (index, group) in ordered_groups.iter().enumerate() {
            let funcs: Vec<_> = group.functions.iter().collect();
            println!("{} group {}", index, format_list(&funcs[..]));
        }
        */

        (self.type_store, self.function_type_info_map, ordered_groups)
    }
}