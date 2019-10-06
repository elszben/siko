use crate::check_context::CheckContext;
use crate::class_processor::ClassProcessor;
use crate::error::Error;
use crate::error::TypecheckError;
use crate::expr_processor::ExprProcessor;
use crate::function_dep_processor::FunctionDependencyProcessor;
use crate::function_processor::FunctionProcessor;
use crate::type_store::TypeStore;
use siko_constants::LIST_NAME;
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
        let check_context = Rc::new(RefCell::new(CheckContext::new(
            program.type_instance_resolver.clone(),
        )));

        let type_store = TypeStore::new(
            program.get_named_type("Data.List", LIST_NAME),
            check_context.clone(),
        );

        let class_processor = ClassProcessor::new(type_store, check_context.clone());

        let (type_store, class_member_type_info_map) =
            class_processor.process_classes(program, &mut errors);

        let function_processor = FunctionProcessor::new(type_store);

        let (type_store, function_type_info_map, record_type_info_map, variant_type_info_map) =
            function_processor.process_functions(program, &mut errors, &class_member_type_info_map);

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        let function_dep_processor =
            FunctionDependencyProcessor::new(type_store, function_type_info_map);

        let (type_store, function_type_info_map, ordered_dep_groups) =
            function_dep_processor.process_functions(program);

        self.check_main(program, &mut errors);

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        let mut expr_processor = ExprProcessor::new(
            type_store,
            function_type_info_map,
            record_type_info_map,
            variant_type_info_map,
            class_member_type_info_map,
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

        expr_processor.check_undefined_generics(&mut errors);

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        expr_processor.export_expr_types();
        expr_processor.export_func_types();
        expr_processor.export_class_member_types();

        Ok(())
    }
}
