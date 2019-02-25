use crate::error::Error;
use crate::ir::program::Program;

pub struct Typechecker {}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {}
    }

    pub fn check(&self, program: &Program) -> Result<(), Error> {
        for (id, function) in &program.functions {
            println!("Type checking function {:?} {:?}", id, function);
        }
        Ok(())
    }
}
