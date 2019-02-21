use std::collections::BTreeSet;

pub struct Environment<'a> {
    variables: BTreeSet<String>,
    parent: Option<&'a Environment<'a>>,
    level: usize,
}

impl<'a> Environment<'a> {
    pub fn new() -> Environment<'a> {
        Environment {
            variables: BTreeSet::new(),
            parent: None,
            level: 0,
        }
    }

    pub fn add(&mut self, var: String) {
        self.variables.insert(var);
    }

    pub fn is_valid(&self, var: &String) -> (bool, usize) {
        if self.variables.get(var).is_some() {
            return (true, self.level);
        } else {
            if let Some(parent) = self.parent {
                parent.is_valid(var)
            } else {
                (false, 0)
            }
        }
    }

    pub fn child(parent: &'a Environment<'a>) -> Environment<'a> {
        Environment {
            variables: BTreeSet::new(),
            parent: Some(parent),
            level: parent.level + 1,
        }
    }

    pub fn level(&self) -> usize {
        self.level
    }
}
