use siko_constants;
use crate::error::Error;
use siko_ir::function::FunctionInfo;
use siko_ir::program::Program;
use crate::error::TypecheckError;
use crate::expr_processor::ExprProcessor;
use crate::function_dep_processor::FunctionDependencyProcessor;
use crate::function_processor::FunctionProcessor;

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
                    if info.module == siko_constants::MAIN_MODULE
                        && info.name == siko_constants::MAIN_FUNCTION
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

        let (type_store, function_type_info_map, record_type_info_map, variant_type_info_map) =
            function_processor.process_functions(program, &mut errors);

        let function_dep_processor =
            FunctionDependencyProcessor::new(type_store, function_type_info_map);

        let (type_store, function_type_info_map, ordered_dep_groups) =
            function_dep_processor.process_functions(program);

        let mut expr_processor = ExprProcessor::new(
            type_store,
            function_type_info_map,
            record_type_info_map,
            variant_type_info_map,
        );

        for group in &ordered_dep_groups {
            expr_processor.process_dep_group(program, group, &mut errors);
        }

        //expr_processor.dump_function_types();
        //expr_processor.dump_expression_types(program);

        expr_processor.check_recursive_types(&mut errors);

        self.check_main(program, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::typecheck_err(errors))
        }
    }
}
