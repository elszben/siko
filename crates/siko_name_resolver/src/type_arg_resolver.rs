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
    allow_implicit: bool,
}

impl TypeArgResolver {
    pub fn new(allow_implicit: bool) -> TypeArgResolver {
        TypeArgResolver {
            args: BTreeMap::new(),
            index: Counter::new(),
            allow_implicit: allow_implicit,
        }
    }

    pub fn add_explicit(&mut self, arg: String, constraints: Vec<ClassId>) {
        let index = self.index.next();
        let info = TypeArgInfo {
            index: index,
            constraints: constraints,
        };
        self.args.insert(arg.clone(), info);
    }

    pub fn resolve_arg(&mut self, arg: &String) -> Option<TypeArgInfo> {
        if self.allow_implicit {
            let index = self.index.next();
            let info = TypeArgInfo {
                index: index,
                constraints: Vec::new(),
            };
            let e = self.args.entry(arg.clone()).or_insert(info);
            Some(e.clone())
        } else {
            return self.args.get(arg).cloned();
        }
    }

    pub fn contains(&self, arg: &str) -> bool {
        self.args.contains_key(arg)
    }
}
