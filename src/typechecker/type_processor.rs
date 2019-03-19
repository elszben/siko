use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::function::FunctionId;
use crate::ir::program::Program;
use crate::typechecker::collector::Collector;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use crate::util::format_list_simple;
use std::collections::BTreeMap;

pub struct TypeProcessor<'a> {
    type_store: &'a mut TypeStore,
    function_type_map: &'a BTreeMap<FunctionId, TypeVariable>,
    type_of_exprs: BTreeMap<ExprId, TypeVariable>,
    function_args: BTreeMap<FunctionId, Vec<TypeVariable>>,
    captured_function_args: BTreeMap<FunctionId, Vec<TypeVariable>>,
    function_call_copied_types: BTreeMap<ExprId, Type>,
    function_id: FunctionId,
}

impl<'a> TypeProcessor<'a> {
    pub fn new(
        type_store: &'a mut TypeStore,
        function_type_map: &'a BTreeMap<FunctionId, TypeVariable>,
        function_id: FunctionId,
        args: Vec<TypeVariable>,
    ) -> TypeProcessor<'a> {
        let mut function_args = BTreeMap::new();
        function_args.insert(function_id, args);
        TypeProcessor {
            type_store: type_store,
            function_type_map: function_type_map,
            type_of_exprs: BTreeMap::new(),
            function_args: function_args,
            captured_function_args: BTreeMap::new(),
            function_call_copied_types: BTreeMap::new(),
            function_id: function_id,
        }
    }

    fn get_type_var_for_expr(&self, id: &ExprId) -> TypeVariable {
        self.type_of_exprs
            .get(id)
            .expect("Sub expr type var not found")
            .clone()
    }

    pub fn get_function_type(&mut self, body: &ExprId) -> TypeVariable {
        let args = self
            .function_args
            .get(&self.function_id)
            .expect("Function args not found");
        let body_var = self
            .type_of_exprs
            .get(body)
            .expect("Body expr var not found");
        if args.is_empty() {
            *body_var
        } else {
            let mut type_vars = args.clone();
            type_vars.push(*body_var);
            let function_type = FunctionType::new(type_vars);
            let function_type = Type::Function(function_type);
            let function_type_var = self.type_store.add_var(function_type);
            function_type_var
        }
    }

    fn process_function_call(
        &mut self,
        ty: &Type,
        function_type: &FunctionType,
        args: &[ExprId],
        id: ExprId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
        name: String,
        unified_variables: &mut bool,
    ) {
        let cloned_function_type: Type = match self.function_call_copied_types.get(&id) {
            Some(c) => c.clone(),
            None => {
                let cloned_ty = self.type_store.clone_type(&ty);
                self.function_call_copied_types
                    .insert(id, cloned_ty.clone());
                cloned_ty
            }
        };
        let type_vars = if let Type::Function(ft) = cloned_function_type {
            ft.type_vars
        } else {
            unreachable!();
        };
        if args.len() > type_vars.len() - 1 {
            let location_id = program.get_expr_location(&id);
            let err = TypecheckError::TooManyArguments(
                location_id,
                name,
                type_vars.len() - 1,
                args.len(),
            );
            errors.push(err);
        } else {
            let mut mismatch = false;
            for (index, arg) in args.iter().enumerate() {
                let arg_var = self.get_type_var_for_expr(arg);
                let type_var = type_vars[index];
                if !self
                    .type_store
                    .unify(&arg_var, &type_var, unified_variables)
                {
                    mismatch = true;
                    break;
                }
            }
            if mismatch {
                let location_id = program.get_expr_location(&id);
                let mut arg_types = Vec::new();
                for arg in args {
                    let arg_var = self.get_type_var_for_expr(arg);
                    let ty = self.type_store.get_resolved_type_string(&arg_var);
                    arg_types.push(ty);
                }
                let arg_types = format_list_simple(&arg_types[..]);
                let func_type = function_type.as_string(self.type_store);
                let err =
                    TypecheckError::FunctionArgumentMismatch(location_id, arg_types, func_type);
                errors.push(err);
            } else {
                let call_var = self.get_type_var_for_expr(&id);
                let rest: Vec<TypeVariable> = type_vars[args.len()..].to_vec();
                let result_var = if rest.len() == 1 {
                    rest[0]
                } else {
                    let closure_type = FunctionType::new(rest);
                    let ty = Type::Function(closure_type);
                    self.type_store.add_var(ty)
                };
                if !self
                    .type_store
                    .unify(&call_var, &result_var, unified_variables)
                {
                    let location_id = program.get_expr_location(&id);
                    let call_type = self.type_store.get_resolved_type_string(&call_var);
                    let result_type = self.type_store.get_resolved_type_string(&result_var);
                    let err = TypecheckError::TypeMismatch(location_id, call_type, result_type);
                    errors.push(err);
                }
            }
        }
    }

    pub fn check_constraints(&mut self, program: &Program, errors: &mut Vec<TypecheckError>) {
        let mut unified_variables = true;
        while unified_variables && errors.is_empty() {
            unified_variables = false;
            self.check_constraints_inner(program, errors, &mut unified_variables, false);
        }
        if errors.is_empty() {
            self.check_constraints_inner(program, errors, &mut unified_variables, true);
        }
    }

    fn check_constraints_inner(
        &mut self,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
        unified_variables: &mut bool,
        final_round: bool,
    ) {
        for (id, _) in self.type_of_exprs.clone() {
            let expr = program.get_expr(&id);
            match expr {
                Expr::IntegerLiteral(_) => {}
                Expr::FloatLiteral(_) => {}
                Expr::BoolLiteral(_) => {}
                Expr::StringLiteral(_) => {}
                Expr::If(cond, true_branch, false_branch) => {
                    let cond_var = self.get_type_var_for_expr(cond);
                    let true_var = self.get_type_var_for_expr(true_branch);
                    let false_var = self.get_type_var_for_expr(false_branch);
                    let cond_ty = self.type_store.get_type(&cond_var);
                    if cond_ty != Type::Bool {
                        let var = self.type_store.add_var(Type::Bool);
                        if !self.type_store.unify(&var, &cond_var, unified_variables) {
                            let location_id = program.get_expr_location(cond);
                            let cond_ty = self.type_store.get_resolved_type_string(&cond_var);
                            let bool_ty = format!("{}", Type::Bool);
                            let err = TypecheckError::TypeMismatch(location_id, bool_ty, cond_ty);
                            errors.push(err);
                        }
                    }
                    if !self
                        .type_store
                        .unify(&true_var, &false_var, unified_variables)
                    {
                        let location_id = program.get_expr_location(&false_branch);
                        let true_type = self.type_store.get_resolved_type_string(&true_var);
                        let false_type = self.type_store.get_resolved_type_string(&false_var);
                        let err = TypecheckError::TypeMismatch(location_id, true_type, false_type);
                        errors.push(err);
                    }
                }
                Expr::StaticFunctionCall(function_id, args) => {
                    let target_func_type_var = self
                        .function_type_map
                        .get(function_id)
                        .expect("Function type not found");
                    let ty = self.type_store.get_type(target_func_type_var);
                    match &ty {
                        Type::Function(function_type) => {
                            let f = program.get_function(function_id);
                            let name = format!("{}", f.info);
                            self.process_function_call(
                                &ty,
                                function_type,
                                args,
                                id,
                                program,
                                errors,
                                name,
                                unified_variables,
                            );
                        }
                        _ => {
                            if !args.is_empty() {
                                let f = program.get_function(function_id);
                                let name = format!("{}", f.info);
                                let location_id = program.get_expr_location(&id);
                                let err = TypecheckError::TooManyArguments(
                                    location_id,
                                    name,
                                    0,
                                    args.len(),
                                );
                                errors.push(err);
                            } else {
                                let call_var = self.get_type_var_for_expr(&id);
                                if !self.type_store.unify(
                                    &call_var,
                                    target_func_type_var,
                                    unified_variables,
                                ) {
                                    let location_id = program.get_expr_location(&id);
                                    let call_type =
                                        self.type_store.get_resolved_type_string(&call_var);
                                    let func_type = self
                                        .type_store
                                        .get_resolved_type_string(target_func_type_var);
                                    let err = TypecheckError::TypeMismatch(
                                        location_id,
                                        call_type,
                                        func_type,
                                    );
                                    errors.push(err);
                                }
                            }
                        }
                    }
                }
                Expr::Tuple(_) => {}
                Expr::Do(_) => {}
                Expr::Bind(_, _) => {}
                Expr::ExprValue(_) => {}
                Expr::DynamicFunctionCall(func_expr_id, args) => {
                    let type_var = self.get_type_var_for_expr(func_expr_id);
                    let ty = self.type_store.get_type(&type_var);
                    let resolved_type = self.type_store.get_resolved_type_string(&type_var);
                    let name = format!("closure({})", resolved_type);
                    match &ty {
                        Type::Function(function_type) => {
                            self.process_function_call(
                                &ty,
                                &function_type,
                                args,
                                id,
                                program,
                                errors,
                                name,
                                unified_variables,
                            );
                        }
                        _ => {
                            if final_round {
                                let location_id = program.get_expr_location(&id);
                                let err = TypecheckError::NotCallableType(
                                    location_id,
                                    format!("{}", resolved_type),
                                );
                                errors.push(err);
                            }
                        }
                    }
                }
                Expr::ArgRef(_) => {}
                Expr::LambdaFunction(lambda_id, _) => {
                    let type_var = self.get_type_var_for_expr(&id);
                    let ty = self.type_store.get_type(&type_var);
                    if let Type::Function(function_type) = ty {
                        let return_type_var = function_type.get_return_type();
                        let lambda_info = program.get_function(lambda_id);
                        let body_id = lambda_info.info.body();
                        let body_var = self.get_type_var_for_expr(&body_id);
                        self.type_store
                            .unify(&body_var, &return_type_var, unified_variables);
                    } else {
                        panic!("Type of lambda is not a function {}", ty);
                    }
                }
                Expr::LambdaCapturedArgRef(_) => {}
            }
        }
    }

    pub fn dump_types(&self, program: &Program) {
        for (id, var) in &self.type_of_exprs {
            let expr = program.get_expr(id);
            let ty = self.type_store.get_resolved_type_string(var);
            println!("{},{:?} {} => {}", id, var, expr, ty);
        }
    }
}

impl<'a> Collector for TypeProcessor<'a> {
    fn process(&mut self, program: &Program, expr: &Expr, id: ExprId) {
        match expr {
            Expr::IntegerLiteral(_) => {
                let ty = Type::Int;
                let var = self.type_store.add_var(ty);
                self.type_of_exprs.insert(id, var);
            }
            Expr::FloatLiteral(_) => {
                let ty = Type::Float;
                let var = self.type_store.add_var(ty);
                self.type_of_exprs.insert(id, var);
            }
            Expr::BoolLiteral(_) => {
                let ty = Type::Bool;
                let var = self.type_store.add_var(ty);
                self.type_of_exprs.insert(id, var);
            }
            Expr::StringLiteral(_) => {
                let ty = Type::String;
                let var = self.type_store.add_var(ty);
                self.type_of_exprs.insert(id, var);
            }
            Expr::If(_, true_branch, _) => {
                let true_var = self.get_type_var_for_expr(true_branch);
                self.type_of_exprs.insert(id, true_var);
            }
            Expr::StaticFunctionCall(_, _) => {
                let ty = self.type_store.get_unique_type_arg_type();
                let result_var = self.type_store.add_var(ty);
                self.type_of_exprs.insert(id, result_var);
            }
            Expr::Tuple(items) => {
                let items: Vec<_> = items
                    .iter()
                    .map(|i| self.get_type_var_for_expr(i))
                    .collect();
                let ty = Type::Tuple(items);
                let var = self.type_store.add_var(ty);
                self.type_of_exprs.insert(id, var);
            }
            Expr::Do(items) => {
                let last = items.last().expect("Empty do");
                let var = self.get_type_var_for_expr(last);
                self.type_of_exprs.insert(id, var);
            }
            Expr::Bind(_, _) => {
                let ty = Type::Tuple(vec![]);
                let var = self.type_store.add_var(ty);
                self.type_of_exprs.insert(id, var);
            }
            Expr::ExprValue(expr_id) => {
                let var = self.get_type_var_for_expr(expr_id);
                self.type_of_exprs.insert(id, var);
            }
            Expr::DynamicFunctionCall(_, _) => {
                let ty = self.type_store.get_unique_type_arg_type();
                let result_var = self.type_store.add_var(ty);
                self.type_of_exprs.insert(id, result_var);
            }
            Expr::ArgRef(index) => {
                let arg_var =
                    self.function_args.get(&index.id).expect("Missing arg set")[index.index];
                self.type_of_exprs.insert(id, arg_var);
            }
            Expr::LambdaFunction(lambda_id, captures) => {
                let captured_vars: Vec<_> = captures
                    .iter()
                    .map(|c| self.get_type_var_for_expr(c))
                    .collect();
                self.captured_function_args
                    .insert(*lambda_id, captured_vars);
                let lambda_info = program.get_function(lambda_id);
                let mut args = Vec::new();
                let mut type_vars = Vec::new();
                for _ in 0..lambda_info.arg_count {
                    let ty = self.type_store.get_unique_type_arg_type();
                    let var = self.type_store.add_var(ty);
                    type_vars.push(var);
                    args.push(var);
                }
                let lambda_result_type = self.type_store.get_unique_type_arg_type();
                let lambda_result_type_var = self.type_store.add_var(lambda_result_type);
                type_vars.push(lambda_result_type_var);
                self.function_args.insert(*lambda_id, args);
                let lambda_function_type = FunctionType::new(type_vars);
                let ty = Type::Function(lambda_function_type);
                let result_var = self.type_store.add_var(ty);
                self.type_of_exprs.insert(id, result_var);
            }
            Expr::LambdaCapturedArgRef(arg_ref) => {
                let var = self
                    .captured_function_args
                    .get(&arg_ref.id)
                    .expect("Missing lambda arg set")[arg_ref.index];
                self.type_of_exprs.insert(id, var);
            }
        }
    }
}
