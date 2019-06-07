use crate::util::Counter;
use std::collections::BTreeMap;

pub struct TypeArgResolver {
    args: BTreeMap<String, (usize, bool)>,
    index: Counter,
    implicit: bool,
}

impl TypeArgResolver {
    pub fn new() -> TypeArgResolver {
        TypeArgResolver {
            args: BTreeMap::new(),
            index: Counter::new(),
            implicit: false,
        }
    }

    pub fn reset_context(&mut self) {
        self.implicit = false;
        self.args.clear();
    }

    pub fn add_explicit_arg(&mut self, arg: String) {
        let index = self.index.next();
        self.args.insert(arg, (index, false));
    }

    pub fn resolve_arg(&mut self, arg: &String) -> Option<usize> {
        if self.implicit {
            let index = self.index.next();
            let e = self.args.entry(arg.clone()).or_insert((index, true));
            Some(e.0)
        } else {
            if let Some(entry) = self.args.get_mut(arg) {
                entry.1 = true;
                Some(entry.0)
            } else {
                None
            }
        }
    }
}
