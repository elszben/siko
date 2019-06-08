use siko_util::Counter;
use std::collections::BTreeMap;

pub struct TypeArgResolver {
    args: BTreeMap<String, usize>,
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

    pub fn add_explicit(&mut self, arg: String) {
        let index = self.index.next();
        self.args.insert(arg.clone(), index);
    }

    pub fn resolve_arg(&mut self, arg: &String) -> Option<usize> {
        if self.allow_implicit {
            let index = self.index.next();
            let e = self.args.entry(arg.clone()).or_insert(index);
            Some(*e)
        } else {
            return self.args.get(arg).copied();
        }
    }

    pub fn contains(&self, arg: &str) -> bool {
        self.args.contains_key(arg)
    }
}
