use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::expr::FunctionArgumentRef;
use crate::ir::function::FunctionId;
use crate::ir::program::Program;
use crate::location_info::item::LocationId;
use crate::typechecker::common::FunctionTypeInfo;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use crate::util::format_list;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

pub fn create_general_function_type(
    arg_count: usize,
    args: &mut Vec<TypeVariable>,
    type_store: &mut TypeStore,
) -> (TypeVariable, TypeVariable) {
    if arg_count > 0 {
        let from_var = type_store.get_new_type_var();
        args.push(from_var);
        let (to_var, result) = create_general_function_type(arg_count - 1, args, type_store);
        let func_ty = Type::Function(FunctionType::new(from_var, to_var));
        let func_var = type_store.add_type(func_ty);
        (func_var, result)
    } else {
        let v = type_store.get_new_type_var();
        (v, v)
    }
}

fn report_type_mismatch(
    expected: &TypeVariable,
    found: &TypeVariable,
    type_store: &TypeStore,
    expected_id: LocationId,
    found_id: LocationId,
    errors: &mut Vec<TypecheckError>,
) {
    let expected_type = type_store.get_resolved_type_string(&expected);
    let found_type = type_store.get_resolved_type_string(&found);
    let err = TypecheckError::TypeMismatch(expected_id, found_id, expected_type, found_type);
    errors.push(err);
}

fn unify_variables(
    expected: &TypeVariable,
    found: &TypeVariable,
    type_store: &mut TypeStore,
    expected_id: LocationId,
    found_id: LocationId,
    errors: &mut Vec<TypecheckError>,
) {
    if !type_store.unify(&found, &expected) {
        report_type_mismatch(expected, found, type_store, expected_id, found_id, errors);
    }
}

#[derive(Clone)]
struct FunctionCallInfo {
    function_type: TypeVariable,
    result: TypeVariable,
    args: Vec<TypeVariable>,
}

pub struct ExprProcessor {
    type_store: TypeStore,
    expression_type_var_map: BTreeMap<ExprId, TypeVariable>,
    function_call_info_map: BTreeMap<ExprId, FunctionCallInfo>,
    function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
}

impl ExprProcessor {
    pub fn new(
        type_store: TypeStore,
        function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    ) -> ExprProcessor {
        ExprProcessor {
            type_store: type_store,
            expression_type_var_map: BTreeMap::new(),
            function_call_info_map: BTreeMap::new(),
            function_type_info_map: function_type_info_map,
        }
    }

    fn print_type(&self, var: TypeVariable, msg: &str) {
        println!("{} {}", msg, self.type_store.get_resolved_type_string(&var));
    }

    fn create_type_var_for_expr(&mut self, expr_id: ExprId) -> TypeVariable {
        let var = self.type_store.get_new_type_var();
        self.expression_type_var_map.insert(expr_id, var);
        var
    }

    pub fn lookup_type_var_for_expr(&self, expr_id: &ExprId) -> TypeVariable {
        *self
            .expression_type_var_map
            .get(expr_id)
            .expect("Type var for expr not found")
    }

    fn check_static_function_call(
        &mut self,
        expr_id: &ExprId,
        function_id: &FunctionId,
        args: &Vec<ExprId>,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        let (func_type_var, result, gen_args) = match self.function_call_info_map.entry(*expr_id) {
            Entry::Occupied(info) => (
                info.get().function_type,
                info.get().result,
                info.get().args.clone(),
            ),
            Entry::Vacant(entry) => {
                let mut gen_args = Vec::new();
                let (func_type_var, result) =
                    create_general_function_type(args.len(), &mut gen_args, &mut self.type_store);
                let info = FunctionCallInfo {
                    function_type: func_type_var,
                    result: result,
                    args: gen_args,
                };
                entry.insert(info.clone());
                (info.function_type, info.result, info.args)
            }
        };

        let target_function_type_info = self
            .function_type_info_map
            .get(function_id)
            .expect("Function type info not found");
        let cloned = self
            .type_store
            .clone_type_var(target_function_type_info.function_type);
        let expr_location_id = program.get_expr_location(expr_id);
        let mut failed = false;
        if self.type_store.unify(&func_type_var, &cloned) {
            let expr_var = self.lookup_type_var_for_expr(expr_id);
            if self.type_store.unify(&expr_var, &result) {
                for (arg, gen_arg) in args.iter().zip(gen_args.iter()) {
                    let arg_var = self.lookup_type_var_for_expr(arg);
                    if !self.type_store.unify(&arg_var, gen_arg) {
                        failed = true;
                        break;
                    }
                }
            } else {
                if args.is_empty() {
                    report_type_mismatch(
                        &expr_var,
                        &result,
                        &self.type_store,
                        expr_location_id,
                        expr_location_id,
                        errors,
                    );
                    return;
                }
                failed = true;
            }
        } else {
            failed = true;
        }
        if failed {
            let mut arg_strs = Vec::new();
            for arg in args {
                let arg_var = self.lookup_type_var_for_expr(arg);
                let arg_type_str = self.type_store.get_resolved_type_string(&arg_var);
                arg_strs.push(arg_type_str);
            }
            let args_str = format_list(&arg_strs[..]);
            let func_type_str = self
                .type_store
                .get_resolved_type_string(&target_function_type_info.function_type);
            let err =
                TypecheckError::FunctionArgumentMismatch(expr_location_id, args_str, func_type_str);
            errors.push(err);
        }
    }

    fn check_arg_ref(&mut self, expr_id: &ExprId, arg_ref: &FunctionArgumentRef) {
        let function_type_info = self
            .function_type_info_map
            .get(&arg_ref.id)
            .expect("Function type info not found");
        let arg_var = function_type_info.args[arg_ref.index];
        let expr_var = self.lookup_type_var_for_expr(expr_id);
        if !self.type_store.unify(&arg_var, &expr_var) {
            panic!("Non typed argument failed to unify with argref");
        }
    }

    fn check_dynamic_function_call(
        &mut self,
        expr_id: &ExprId,
        callable_expr_id: &ExprId,
        args: &Vec<ExprId>,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        let mut gen_args = Vec::new();
        let (func_type_var, result) =
            create_general_function_type(args.len(), &mut gen_args, &mut self.type_store);
        let callable_expr_var = self.lookup_type_var_for_expr(callable_expr_id);
        let callable_location_id = program.get_expr_location(callable_expr_id);
        unify_variables(
            &func_type_var,
            &callable_expr_var,
            &mut self.type_store,
            callable_location_id,
            callable_location_id,
            errors,
        );
        let expr_var = self.lookup_type_var_for_expr(expr_id);
        let expr_location_id = program.get_expr_location(expr_id);
        unify_variables(
            &expr_var,
            &result,
            &mut self.type_store,
            expr_location_id,
            expr_location_id,
            errors,
        );
        for (arg, gen_arg) in args.iter().zip(gen_args.iter()) {
            let arg_var = self.lookup_type_var_for_expr(arg);
            unify_variables(
                &arg_var,
                &gen_arg,
                &mut self.type_store,
                expr_location_id,
                expr_location_id,
                errors,
            );
        }
    }

    fn check_do(
        &mut self,
        expr_id: &ExprId,
        exprs: &Vec<ExprId>,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        let last_expr_id = exprs[exprs.len() - 1];
        let expr_var = self.lookup_type_var_for_expr(expr_id);
        let last_expr_var = self.lookup_type_var_for_expr(&last_expr_id);
        let expr_location_id = program.get_expr_location(expr_id);
        let last_expr_location_id = program.get_expr_location(&last_expr_id);
        unify_variables(
            &expr_var,
            &last_expr_var,
            &mut self.type_store,
            expr_location_id,
            last_expr_location_id,
            errors,
        );
    }

    fn check_bind(
        &mut self,
        expr_id: &ExprId,
        rhs_expr_id: &ExprId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        let expr_var = self.lookup_type_var_for_expr(expr_id);
        let rhs_expr_var = self.lookup_type_var_for_expr(&rhs_expr_id);
        let expr_location_id = program.get_expr_location(expr_id);
        let last_expr_location_id = program.get_expr_location(&rhs_expr_id);
        unify_variables(
            &expr_var,
            &rhs_expr_var,
            &mut self.type_store,
            expr_location_id,
            last_expr_location_id,
            errors,
        );
    }

    fn check_expr_value(
        &mut self,
        expr_id: &ExprId,
        ref_expr_id: &ExprId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        let expr_var = self.lookup_type_var_for_expr(expr_id);
        let ref_expr_var = self.lookup_type_var_for_expr(&ref_expr_id);
        let expr_location_id = program.get_expr_location(expr_id);
        let ref_expr_location_id = program.get_expr_location(&ref_expr_id);
        unify_variables(
            &expr_var,
            &ref_expr_var,
            &mut self.type_store,
            expr_location_id,
            ref_expr_location_id,
            errors,
        );
    }

    fn check_if(
        &mut self,
        expr_id: &ExprId,
        cond_expr: &ExprId,
        true_branch_expr: &ExprId,
        false_branch_expr: &ExprId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        let bool_var = self.type_store.add_type(Type::Bool);
        let cond_var = self.lookup_type_var_for_expr(&cond_expr);
        let expr_location_id = program.get_expr_location(cond_expr);
        unify_variables(
            &bool_var,
            &cond_var,
            &mut self.type_store,
            expr_location_id,
            expr_location_id,
            errors,
        );
        let true_var = self.lookup_type_var_for_expr(&true_branch_expr);
        let true_expr_location_id = program.get_expr_location(true_branch_expr);
        let false_var = self.lookup_type_var_for_expr(&false_branch_expr);
        let false_expr_location_id = program.get_expr_location(false_branch_expr);
        unify_variables(
            &true_var,
            &false_var,
            &mut self.type_store,
            true_expr_location_id,
            false_expr_location_id,
            errors,
        );
        let expr_var = self.lookup_type_var_for_expr(expr_id);
        let expr_location_id = program.get_expr_location(expr_id);
        unify_variables(
            &expr_var,
            &true_var,
            &mut self.type_store,
            expr_location_id,
            true_expr_location_id,
            errors,
        );
    }

    fn check_tuple(
        &mut self,
        expr_id: &ExprId,
        exprs: &Vec<ExprId>,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        let expr_vars: Vec<_> = exprs
            .iter()
            .map(|e| self.lookup_type_var_for_expr(e))
            .collect();
        let expr_var = self.lookup_type_var_for_expr(expr_id);
        let expr_location_id = program.get_expr_location(expr_id);
        let tuple_ty = Type::Tuple(expr_vars);
        let tuple_var = self.type_store.add_type(tuple_ty);
        unify_variables(
            &expr_var,
            &tuple_var,
            &mut self.type_store,
            expr_location_id,
            expr_location_id,
            errors,
        );
    }

    fn check_tuple_field_access(
        &mut self,
        expr_id: &ExprId,
        index: usize,
        tuple_expr: &ExprId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        //TODO
        let tuple_expr_location_id = program.get_expr_location(tuple_expr);
        let tuple_var = self.lookup_type_var_for_expr(tuple_expr);
        let var = self.type_store.add_type(Type::TupleFieldIndexable(index));
        unify_variables(
            &var,
            &tuple_var,
            &mut self.type_store,
            tuple_expr_location_id,
            tuple_expr_location_id,
            errors,
        );
    }

    pub fn check_body_and_result(&mut self, program: &Program, errors: &mut Vec<TypecheckError>) {
        for (_, function_type_info) in &self.function_type_info_map {
            let body = if let Some(body) = function_type_info.body {
                body
            } else {
                continue;
            };
            let location_id = program.get_expr_location(&body);
            let body_var = self.lookup_type_var_for_expr(&body);
            if let Some(locations) = &function_type_info.signature_location {
                unify_variables(
                    &function_type_info.result,
                    &body_var,
                    &mut self.type_store,
                    locations.return_location_id,
                    location_id,
                    errors,
                );
            } else {
                unify_variables(
                    &function_type_info.result,
                    &body_var,
                    &mut self.type_store,
                    location_id,
                    location_id,
                    errors,
                );
            }
        }
    }

    pub fn process_expr_and_create_vars(&mut self, program: &Program) {
        for (expr_id, expr_info) in &program.exprs {
            //println!("Processing {} {}", expr_id, expr_info.expr);
            match &expr_info.expr {
                Expr::IntegerLiteral(_) => {
                    let var = self.type_store.add_type(Type::Int);
                    self.expression_type_var_map.insert(*expr_id, var);
                }
                Expr::BoolLiteral(_) => {
                    let var = self.type_store.add_type(Type::Bool);
                    self.expression_type_var_map.insert(*expr_id, var);
                }
                Expr::StringLiteral(_) => {
                    let var = self.type_store.add_type(Type::String);
                    self.expression_type_var_map.insert(*expr_id, var);
                }
                Expr::FloatLiteral(_) => {
                    let var = self.type_store.add_type(Type::Float);
                    self.expression_type_var_map.insert(*expr_id, var);
                }
                _ => {
                    self.create_type_var_for_expr(*expr_id);
                }
            }
        }
    }

    pub fn check_exprs(&mut self, program: &Program, errors: &mut Vec<TypecheckError>) {
        for (expr_id, expr_info) in &program.exprs {
            // println!("Checking {} {}", expr_id, expr_info.expr);
            match &expr_info.expr {
                Expr::IntegerLiteral(_) => {}
                Expr::BoolLiteral(_) => {}
                Expr::StringLiteral(_) => {}
                Expr::FloatLiteral(_) => {}
                Expr::StaticFunctionCall(function_id, args) => {
                    self.check_static_function_call(expr_id, function_id, args, program, errors);
                }
                Expr::ArgRef(arg_ref) => {
                    self.check_arg_ref(expr_id, arg_ref);
                }
                Expr::DynamicFunctionCall(callable_expr_id, args) => {
                    self.check_dynamic_function_call(
                        expr_id,
                        callable_expr_id,
                        args,
                        program,
                        errors,
                    );
                }
                Expr::Do(exprs) => {
                    self.check_do(expr_id, exprs, program, errors);
                }
                Expr::Bind(_, rhs_expr_id) => {
                    self.check_bind(expr_id, rhs_expr_id, program, errors);
                }
                Expr::ExprValue(ref_expr_id) => {
                    self.check_expr_value(expr_id, ref_expr_id, program, errors);
                }
                Expr::If(cond_expr, true_branch_expr, false_branch_expr) => {
                    self.check_if(
                        expr_id,
                        cond_expr,
                        true_branch_expr,
                        false_branch_expr,
                        program,
                        errors,
                    );
                }
                Expr::Tuple(exprs) => {
                    self.check_tuple(expr_id, exprs, program, errors);
                }
                Expr::TupleFieldAccess(index, tuple_expr) => {
                    self.check_tuple_field_access(expr_id, *index, tuple_expr, program, errors);
                }
                _ => {
                    panic!("Unimplemented expr {}", expr_info.expr);
                }
            }
        }
    }

    pub fn dump_everything(&self, program: &Program) {
        for (id, info) in &self.function_type_info_map {
            println!(
                "{}/{}: {}",
                id,
                info.displayed_name,
                self.type_store
                    .get_resolved_type_string(&info.function_type)
            );
        }

        for (expr_id, expr_info) in &program.exprs {
            let var = self.lookup_type_var_for_expr(expr_id);
            println!(
                "Expr: {}: {} -> {}",
                expr_id,
                expr_info.expr,
                self.type_store.get_resolved_type_string(&var)
            );
        }
    }

    pub fn check_constraints(&mut self, program: &Program, errors: &mut Vec<TypecheckError>) {
        let mut run = true;
        let mut loop_count = 20;
        while run && errors.is_empty() && loop_count > 0 {
            self.check_exprs(program, errors);
            self.check_body_and_result(program, errors);
            loop_count -= 1;
            let primary_modified = self.type_store.progress_checker.get_and_unset();
            if !primary_modified {
                run = false;
            }
        }
        assert!(loop_count > 0);
    }
}
