use siko_ir::class::ClassId;
use siko_util::Counter;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct TypeArgInfo {
    pub index: usize,
    pub constraints: Vec<ClassId>,
}

pub struct TypeArgResolver {
    args: BTreeMap<String, TypeArgInfo>,
    index: Counter,
}

impl TypeArgResolver {
    pub fn new() -> TypeArgResolver {
        TypeArgResolver {
            args: BTreeMap::new(),
            index: Counter::new(),
        }
    }

    pub fn add_explicit(&mut self, arg: String, constraints: Vec<ClassId>) -> usize {
        let index = self.index.next();
        let info = TypeArgInfo {
            index: index,
            constraints: constraints,
        };
        self.args.insert(arg.clone(), info);
        index
    }

    pub fn add_constraint(&mut self, arg: &String, constraint: ClassId) -> bool {
        if let Some(info) = self.args.get_mut(arg) {
            info.constraints.push(constraint);
            info.constraints.sort();
            info.constraints.dedup();
            true
        } else {
            false
        }
    }

    pub fn resolve_arg(&self, arg: &String) -> Option<TypeArgInfo> {
        self.args.get(arg).cloned()
    }

    pub fn contains(&self, arg: &str) -> bool {
        self.args.contains_key(arg)
    }
}
