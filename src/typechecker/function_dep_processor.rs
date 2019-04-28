use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::function::FunctionId;
use crate::ir::program::Program;
use crate::typechecker::common::DependencyGroup;
use crate::typechecker::common::FunctionTypeInfo;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::walker::walk_expr;
use crate::typechecker::walker::Visitor;
use crate::util::format_list;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

struct UntypedFunctionDependency {
    untyped_deps: BTreeSet<FunctionId>,
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
    fn visit(&mut self, _: ExprId, expr: &Expr) {
        match expr {
            Expr::StaticFunctionCall(id, _) => {
                self.used_functions.insert(*id);
            }
            _ => {}
        }
    }
}

pub struct FunctionDependencyProcessor {
    type_store: TypeStore,
    function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    untyped_deps: BTreeMap<FunctionId, UntypedFunctionDependency>,
}

impl FunctionDependencyProcessor {
    pub fn new(
        type_store: TypeStore,
        function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    ) -> FunctionDependencyProcessor {
        FunctionDependencyProcessor {
            type_store: type_store,
            function_type_info_map: function_type_info_map,
            untyped_deps: BTreeMap::new(),
        }
    }

    fn collect_typed_deps(&mut self, program: &Program) -> Vec<DependencyGroup> {
        let mut groups = Vec::new();
        for (id, type_info) in &self.function_type_info_map {
            if type_info.signature_location.is_none() {
                continue;
            }
            let body = if let Some(body) = type_info.body {
                body
            } else {
                continue;
            };
            let mut collector = DependencyCollector::new();
            walk_expr(&body, program, &mut collector);
            let deps: Vec<_> = collector.used_functions.into_iter().collect();
            let mut deps: BTreeSet<_> = deps
                .iter()
                .filter(|dep_id| {
                    let dep_func = program.functions.get(dep_id).expect("Function not found");
                    dep_func.is_lambda()
                })
                .map(|id| *id)
                .collect();
            deps.insert(*id);
            let mut group = DependencyGroup::new();
            group.functions = deps;
            groups.push(group);
        }
        groups
    }

    fn collect_untyped_deps(&mut self, program: &Program) {
        for (id, type_info) in &self.function_type_info_map {
            if type_info.signature_location.is_some() {
                continue;
            }
            if let Some(body) = type_info.body {
                let mut collector = DependencyCollector::new();
                walk_expr(&body, program, &mut collector);
                let deps: Vec<_> = collector.used_functions.into_iter().collect();
                //println!("deps {}", format_list(&deps[..]));
                let untyped_deps: BTreeSet<_> = deps
                    .iter()
                    .filter(|dep_id| {
                        let dep_info = self
                            .function_type_info_map
                            .get(dep_id)
                            .expect("type info not found");
                        dep_info.signature_location.is_none()
                    })
                    .map(|id| *id)
                    .collect();
                self.untyped_deps.insert(
                    *id,
                    UntypedFunctionDependency {
                        untyped_deps: untyped_deps,
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
        let deps = self.untyped_deps.get(user).expect("dep info not found");
        if deps.untyped_deps.contains(used_function) {
            return true;
        } else {
            for dep in &deps.untyped_deps {
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
        self.collect_untyped_deps(program);

        let mut group_table = GroupTable::new();

        for (index, (id, _)) in self.untyped_deps.iter().enumerate() {
            group_table.table.insert(*id, index);
        }

        for (id, d) in &self.untyped_deps {
            for dep in &d.untyped_deps {
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
                    let deps = self.untyped_deps.get(function).expect("dep not found");
                    for dep in &deps.untyped_deps {
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

        let typed_groups = self.collect_typed_deps(program);
        ordered_groups.extend(typed_groups);

        for (index, group) in ordered_groups.iter().enumerate() {
            let funcs: Vec<_> = group.functions.iter().collect();
            println!("{} group {}", index, format_list(&funcs[..]));
        }

        (self.type_store, self.function_type_info_map, ordered_groups)
    }
}
