use crate::check_context::CheckContext;
use crate::class_processor::ClassProcessor;
use crate::error::Error;
use crate::error::TypecheckError;
use crate::expr_processor::ExprProcessor;
use crate::function_dep_processor::FunctionDependencyProcessor;
use crate::function_processor::FunctionProcessor;
use siko_constants;
use siko_ir::function::FunctionInfo;
use siko_ir::program::Program;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Typechecker {}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {}
    }

    fn check_main(&self, program: &Program, errors: &mut Vec<TypecheckError>) {
        let mut main_found = false;

        for (_, function) in &program.functions.items {
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

    pub fn check(&mut self, program: &mut Program) -> Result<(), Error> {
        let mut errors = Vec::new();

        let check_context = Rc::new(RefCell::new(CheckContext::new()));

        let function_processor = FunctionProcessor::new(
            program.builtin_types.list_id.unwrap(),
            check_context.clone(),
        );

        let (type_store, function_type_info_map, record_type_info_map, variant_type_info_map) =
            function_processor.process_functions(program, &mut errors);

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        let function_dep_processor =
            FunctionDependencyProcessor::new(type_store, function_type_info_map);

        let (type_store, function_type_info_map, ordered_dep_groups) =
            function_dep_processor.process_functions(program);

        let class_processor = ClassProcessor::new(type_store, check_context.clone());

        let (type_store, class_type_info_map) =
            class_processor.process_classes(program, &mut errors);

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        let mut expr_processor = ExprProcessor::new(
            type_store,
            function_type_info_map,
            record_type_info_map,
            variant_type_info_map,
            class_type_info_map,
            program,
        );

        for group in &ordered_dep_groups {
            expr_processor.process_dep_group(group, &mut errors);
        }

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
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
