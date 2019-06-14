use siko_ir::class::ClassId;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct TypeArgConstraintCollection {
    pub items: Vec<(String, Vec<ClassId>)>,
}

impl TypeArgConstraintCollection {
    pub fn new() -> TypeArgConstraintCollection {
        TypeArgConstraintCollection {
            items: Vec::new()
        }
    }
}

pub struct TypeArgConstraintCollector {
    constraints: BTreeMap<String, Vec<ClassId>>,
}

impl TypeArgConstraintCollector {
    pub fn new() -> TypeArgConstraintCollector {
        TypeArgConstraintCollector {
            constraints: BTreeMap::new(),
        }
    }

    pub fn add_empty(&mut self, arg: String) {
        self.constraints.insert(arg, Vec::new());
    }

    pub fn add_constraint(&mut self, arg: String, class_id: ClassId) {
        let classes = self
            .constraints
            .entry(arg.clone())
            .or_insert_with(|| Vec::new());
        classes.push(class_id);
    }

    pub fn get_all_constraints(&self) -> TypeArgConstraintCollection {
        let mut result = TypeArgConstraintCollection::new();
        for (arg, constraints) in &self.constraints {
            result.items.push((arg.clone(), constraints.clone()));
        }
        result
    }
}