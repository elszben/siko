use crate::constants;
use crate::error::Error;
use crate::ir::function::FunctionInfo;
use crate::ir::program::Program;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::expr_processor::ExprProcessor;
use crate::typechecker::function_dep_processor::FunctionDependencyProcessor;
use crate::typechecker::function_processor::FunctionProcessor;

pub struct Typechecker {}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {}
    }

    fn check_main(&self, program: &Program, errors: &mut Vec<TypecheckError>) {
        let mut main_found = false;

        for (_, function) in &program.functions {
            match &function.info {
                FunctionInfo::NamedFunction(info) => {
                    if info.module == constants::MAIN_MODULE
                        && info.name == constants::MAIN_FUNCTION
                    {
                        main_found = true;
                    }
                }
                _ => {}
            }
        }

        if !main_found {
            errors.push(TypecheckError::MainNotFound);
        }
    }

    pub fn check(&mut self, program: &Program) -> Result<(), Error> {
        let mut errors = Vec::new();

        let function_processor = FunctionProcessor::new();

        let (type_store, function_type_info_map) =
            function_processor.process_functions(program, &mut errors);

        let function_dep_processor =
            FunctionDependencyProcessor::new(type_store, function_type_info_map);

        let (type_store, function_type_info_map, ordered_untyped_dep_groups) =
            function_dep_processor.process_functions(program);

        let mut expr_processor = ExprProcessor::new(type_store, function_type_info_map);

        for group in &ordered_untyped_dep_groups {
            expr_processor.process_untyped_dep_group(program, group);
        }

        // expr_processor.dump_everything(program);

        self.check_main(program, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::typecheck_err(errors))
        }
    }
}
