use crate::constants;
use crate::error::Error;
use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::expr::FunctionArgumentRef;
use crate::ir::function::Function;
use crate::ir::function::FunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::function::NamedFunctionInfo;
use crate::ir::program::Program;
use crate::ir::types::TypeSignature;
use crate::ir::types::TypeSignatureId;
use crate::location_info::item::LocationId;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::type_store::ProgressTrackingMode;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use crate::util::format_list;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

#[derive(Clone)]
pub struct ProgressChecker {
    data: Rc<RefCell<bool>>,
}

impl ProgressChecker {
    fn new() -> ProgressChecker {
        ProgressChecker {
            data: Rc::new(RefCell::new(false)),
        }
    }

    pub fn set(&self) {
        let mut d = self.data.borrow_mut();
        *d = true;
    }

    fn get_and_unset(&self) -> bool {
        let mut d = self.data.borrow_mut();
        let r = *d;
        *d = false;
        r
    }
}

struct FunctionSignatureLocation {
    arg_locations: Vec<LocationId>,
    return_location_id: LocationId,
}

struct FunctionTypeInfo {
    displayed_name: String,
    args: Vec<TypeVariable>,
    signature_location: Option<FunctionSignatureLocation>,
    arg_locations: Vec<LocationId>,
    result: TypeVariable,
    function_type: TypeVariable,
    body: Option<ExprId>,
}

impl FunctionTypeInfo {
    fn new(
        displayed_name: String,
        args: Vec<TypeVariable>,
        signature_location: Option<FunctionSignatureLocation>,
        arg_locations: Vec<LocationId>,
        result: TypeVariable,
        function_type: TypeVariable,
        body: Option<ExprId>,
    ) -> FunctionTypeInfo {
        FunctionTypeInfo {
            displayed_name: displayed_name,
            args: args,
            signature_location: signature_location,
            arg_locations: arg_locations,
            result: result,
            function_type: function_type,
            body: body,
        }
    }
}

impl fmt::Display for FunctionTypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut vars = self.args.clone();
        vars.push(self.result);
        let ss: Vec<_> = vars.iter().map(|i| format!("{}", i)).collect();
        write!(f, "{} = {}", self.function_type, ss.join(" -> "))
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
    if !type_store.unify(&found, &expected, ProgressTrackingMode::All) {
        report_type_mismatch(expected, found, type_store, expected_id, found_id, errors);
    }
}

pub struct Typechecker {
    type_store: TypeStore,
    function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    expression_type_var_map: BTreeMap<ExprId, TypeVariable>,
    progress_checker: ProgressChecker,
}

impl Typechecker {
    pub fn new() -> Typechecker {
        let progress_checker = ProgressChecker::new();
        Typechecker {
            type_store: TypeStore::new(progress_checker.clone()),
            function_type_info_map: BTreeMap::new(),
            expression_type_var_map: BTreeMap::new(),
            progress_checker: progress_checker,
        }
    }

    fn create_type_var_for_expr(&mut self, expr_id: ExprId) -> TypeVariable {
        let var = self.type_store.get_new_type_var();
        self.expression_type_var_map.insert(expr_id, var);
        var
    }

    fn lookup_type_var_for_expr(&self, expr_id: &ExprId) -> TypeVariable {
        *self
            .expression_type_var_map
            .get(expr_id)
            .expect("Type var for expr not found")
    }

    fn process_type_signature(
        &mut self,
        type_signature_id: &TypeSignatureId,
        program: &Program,
        arg_map: &mut BTreeMap<usize, TypeVariable>,
        signature_arg_locations: &mut Vec<LocationId>,
        arg_count: usize,
    ) -> (TypeVariable, LocationId) {
        let type_signature = program.get_type_signature(type_signature_id);
        let location_id = program.get_type_signature_location(type_signature_id);
        match type_signature {
            TypeSignature::Bool => {
                let ty = Type::Bool;
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::Int => {
                let ty = Type::Int;
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::String => {
                let ty = Type::String;
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::Nothing => {
                let ty = Type::Nothing;
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::Tuple(items) => {
                let items: Vec<_> = items
                    .iter()
                    .map(|i| {
                        self.process_type_signature(
                            i,
                            program,
                            arg_map,
                            signature_arg_locations,
                            arg_count,
                        )
                    })
                    .map(|(i, _)| i)
                    .collect();
                let ty = Type::Tuple(items);
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::Function(from, to) => {
                let (from_var, _) =
                    self.process_type_signature(from, program, arg_map, signature_arg_locations, 0);
                let (to_var, _) = self.process_type_signature(
                    to,
                    program,
                    arg_map,
                    signature_arg_locations,
                    if arg_count > 0 { arg_count - 1 } else { 0 },
                );
                if arg_count > 0 {
                    let from_location_id = program.get_type_signature_location(from);
                    signature_arg_locations.push(from_location_id);
                }
                let to_location_id = program.get_type_signature_location(to);
                let ty = Type::Function(FunctionType::new(from_var, to_var));
                return (self.type_store.add_type(ty), to_location_id);
            }
            TypeSignature::TypeArgument(index, name) => {
                let var = arg_map.entry(*index).or_insert_with(|| {
                    let arg = self.type_store.get_unique_type_arg();
                    let ty = Type::FixedTypeArgument(arg, name.clone());
                    self.type_store.add_type(ty)
                });
                return (*var, location_id);
            }
            TypeSignature::Named(..) => {
                let ty = Type::Nothing;
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::Variant(..) => {
                let ty = Type::Nothing;
                return (self.type_store.add_type(ty), location_id);
            }
        }
    }

    fn register_typed_function(
        &mut self,
        displayed_name: String,
        named_info: &NamedFunctionInfo,
        arg_locations: Vec<LocationId>,
        type_signature_id: TypeSignatureId,
        function_id: FunctionId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
        body: Option<ExprId>,
    ) {
        let mut arg_map = BTreeMap::new();
        let mut signature_arg_locations = Vec::new();
        let (func_type_var, return_location_id) = self.process_type_signature(
            &type_signature_id,
            program,
            &mut arg_map,
            &mut signature_arg_locations,
            arg_locations.len(),
        );
        /*
        println!(
            "Registering named function {} {} with type {}",
            function_id,
            displayed_name,
            self.type_store.get_resolved_type_string(&func_type_var)
        );
        */
        let ty = self.type_store.get_type(&func_type_var);
        let function_type_info = match ty {
            Type::Function(func_type) => {
                let mut signature_vars = Vec::new();
                func_type.get_arg_types(&self.type_store, &mut signature_vars);
                if signature_vars.len() < arg_locations.len() {
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        named_info.name.clone(),
                        arg_locations.len(),
                        signature_vars.len(),
                        program.get_type_signature_location(&type_signature_id),
                    );
                    errors.push(err);
                    return;
                }
                let signature_location = FunctionSignatureLocation {
                    return_location_id: return_location_id,
                    arg_locations: signature_arg_locations,
                };

                let arg_vars: Vec<_> = signature_vars
                    .iter()
                    .take(arg_locations.len())
                    .cloned()
                    .collect();

                let return_value_var = func_type.get_return_type(&self.type_store, arg_vars.len());
                FunctionTypeInfo::new(
                    displayed_name,
                    arg_vars,
                    Some(signature_location),
                    arg_locations,
                    return_value_var,
                    func_type_var,
                    body,
                )
            }
            _ => {
                if arg_locations.len() > 0 {
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        displayed_name,
                        arg_locations.len(),
                        0,
                        program.get_type_signature_location(&type_signature_id),
                    );
                    errors.push(err);
                    return;
                }
                let signature_location = FunctionSignatureLocation {
                    return_location_id: return_location_id,
                    arg_locations: vec![],
                };
                FunctionTypeInfo::new(
                    named_info.name.clone(),
                    vec![],
                    Some(signature_location),
                    arg_locations,
                    func_type_var,
                    func_type_var,
                    body,
                )
            }
        };
        self.function_type_info_map
            .insert(function_id, function_type_info);
    }

    fn create_general_function_type(
        &mut self,
        arg_count: usize,
        args: &mut Vec<TypeVariable>,
    ) -> (TypeVariable, TypeVariable) {
        if arg_count > 0 {
            let from_var = self.type_store.get_new_type_var();
            args.push(from_var);
            let (to_var, result) = self.create_general_function_type(arg_count - 1, args);
            let func_ty = Type::Function(FunctionType::new(from_var, to_var));
            let func_var = self.type_store.add_type(func_ty);
            (func_var, result)
        } else {
            let v = self.type_store.get_new_type_var();
            (v, v)
        }
    }

    fn register_untyped_function(
        &mut self,
        name: String,
        id: FunctionId,
        function: &Function,
        body: ExprId,
    ) {
        let mut args = Vec::new();

        let (func_type_var, result) =
            self.create_general_function_type(function.arg_locations.len(), &mut args);
        let function_type_info = FunctionTypeInfo::new(
            name,
            args,
            None,
            function.arg_locations.clone(),
            result,
            func_type_var,
            Some(body),
        );
        self.function_type_info_map.insert(id, function_type_info);
    }

    fn process_expr_and_create_vars(&mut self, program: &Program) {
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

    fn print_type(&self, var: TypeVariable, msg: &str) {
        println!("{} {}", msg, self.type_store.get_resolved_type_string(&var));
    }

    fn check_static_function_call(
        &mut self,
        expr_id: &ExprId,
        function_id: &FunctionId,
        args: &Vec<ExprId>,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        let mut gen_args = Vec::new();
        let (func_type_var, result) = self.create_general_function_type(args.len(), &mut gen_args);
        let target_function_type_info = self
            .function_type_info_map
            .get(function_id)
            .expect("Function type info not found");
        let cloned = self
            .type_store
            .clone_type_var(target_function_type_info.function_type);
        let expr_location_id = program.get_expr_location(expr_id);
        let mut failed = false;
        if self
            .type_store
            .unify(&func_type_var, &cloned, ProgressTrackingMode::None)
        {
            let expr_var = self.lookup_type_var_for_expr(expr_id);
            if self
                .type_store
                .unify(&expr_var, &result, ProgressTrackingMode::PrimaryOnly)
            {
                for (arg, gen_arg) in args.iter().zip(gen_args.iter()) {
                    let arg_var = self.lookup_type_var_for_expr(arg);
                    if !self
                        .type_store
                        .unify(&arg_var, gen_arg, ProgressTrackingMode::PrimaryOnly)
                    {
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
        if !self
            .type_store
            .unify(&arg_var, &expr_var, ProgressTrackingMode::All)
        {
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
        let (func_type_var, result) = self.create_general_function_type(args.len(), &mut gen_args);
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
        let expr_var = self.lookup_type_var_for_expr(expr_id);
        let expr_location_id = program.get_expr_location(expr_id);
        let tuple_var = self.lookup_type_var_for_expr(tuple_expr);
        let tuple_ty = self.type_store.get_type(&tuple_var);
        if let Type::Tuple(items) = tuple_ty {
            if items.len() > index {
                unify_variables(
                    &expr_var,
                    &items[index],
                    &mut self.type_store,
                    expr_location_id,
                    expr_location_id,
                    errors,
                );
                return;
            }
        }
        let expected_type = format!("<tuple with at least {} item(s)>", index + 1);
        let found_type = self.type_store.get_resolved_type_string(&tuple_var);
        let err = TypecheckError::TypeMismatch(
            expr_location_id,
            expr_location_id,
            expected_type,
            found_type,
        );
        errors.push(err);
    }

    fn check_exprs(&mut self, program: &Program, errors: &mut Vec<TypecheckError>, phase: usize) {
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
                    if phase == 1 {
                        self.check_tuple_field_access(expr_id, *index, tuple_expr, program, errors);
                    }
                }
                _ => {
                    panic!("Unimplemented expr {}", expr_info.expr);
                }
            }
        }
    }

    fn check_body_and_result(&mut self, program: &Program, errors: &mut Vec<TypecheckError>) {
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

    fn check_constraints(&mut self, program: &Program, errors: &mut Vec<TypecheckError>) {
        let mut run = true;
        let mut loop_count = 10;
        let mut phase = 0;
        while run && errors.is_empty() && loop_count > 0 {
            self.check_exprs(program, errors, phase);
            self.check_body_and_result(program, errors);
            loop_count -= 1;
            let primary_modified = self.progress_checker.get_and_unset();
            if !primary_modified {
                if phase == 2 {
                    run = false;
                } else {
                    phase += 1;
                }
            }
        }
        assert!(loop_count > 0);
    }

    fn dump_everything(&self, program: &Program) {
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

    fn process_functions(&mut self, program: &Program, errors: &mut Vec<TypecheckError>) {
        for (id, function) in &program.functions {
            let displayed_name = format!("{}", function.info);
            match &function.info {
                FunctionInfo::RecordConstructor(_) => {}
                FunctionInfo::VariantConstructor(_) => {}
                FunctionInfo::Lambda(_) => {}
                FunctionInfo::NamedFunction(i) => match i.type_signature {
                    Some(type_signature) => {
                        self.register_typed_function(
                            displayed_name,
                            i,
                            function.arg_locations.clone(),
                            type_signature,
                            *id,
                            program,
                            errors,
                            i.body,
                        );
                    }
                    None => match i.body {
                        Some(body) => {
                            self.register_untyped_function(displayed_name, *id, function, body);
                        }
                        None => {
                            let err = TypecheckError::UntypedExternFunction(
                                i.name.clone(),
                                i.location_id,
                            );
                            errors.push(err)
                        }
                    },
                },
            }
        }
    }

    pub fn check(&mut self, program: &Program) -> Result<(), Error> {
        let mut errors = Vec::new();

        self.process_functions(program, &mut errors);

        self.process_expr_and_create_vars(program);

        self.check_constraints(program, &mut errors);

        //self.dump_everything(program);

        self.check_main(program, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::typecheck_err(errors))
        }
    }
}
