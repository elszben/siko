use super::function_type::FunctionTypeStore;
use super::type_constraint::FunctionHeader;
use super::type_constraint::TypeConstraint;
use super::type_store::TypeStore;
use super::type_store::UnificationResultCollector;
use crate::error::Error;

#[derive(Debug)]
pub struct ConstraintStore {
    constraints: Vec<TypeConstraint>,
    function_header: Option<FunctionHeader>,
}

impl ConstraintStore {
    pub fn new() -> ConstraintStore {
        ConstraintStore {
            constraints: Vec::new(),
            function_header: None,
        }
    }

    pub fn add(&mut self, constraint: TypeConstraint) {
        self.constraints.push(constraint);
    }

    pub fn dump(&self) {
        for c in &self.constraints {
            println!("{:?}", c);
        }
    }

    pub fn process(
        &mut self,
        type_store: &mut TypeStore,
        function_type_store: &FunctionTypeStore,
    ) -> Result<(), Error> {
        let mut failed = false;
        for c in &mut self.constraints {
            c.prepare(type_store, function_type_store);
        }
        for c in &mut self.constraints {
            let mut result = UnificationResultCollector::new();
            c.check(type_store, &mut result);
            if result.is_failed() {
                for e in result.errors {
                    println!("Error {}", e);
                }
                failed = true;
                break;
            }
        }
        if failed {
            let s = format!("Constraint failed during typecheck");
            let err = Error::typecheck_err(s);
            return Err(err);
        }
        Ok(())
    }

    pub fn set_function_header(&mut self, function_header: FunctionHeader) {
        self.function_header = Some(function_header);
    }

    pub fn check_header(&mut self, type_store: &TypeStore) -> Result<(), Error> {
        return self
            .function_header
            .as_mut()
            .expect("Missing function header")
            .check(type_store);
    }

    pub fn get_function_header(&self) -> &FunctionHeader {
        self.function_header
            .as_ref()
            .expect("Missing function header")
    }
}
