use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::function::FunctionId;
use crate::ir::program::Program;
use crate::location_info::item::LocationId;
use crate::typechecker::common::create_general_function_type;
use crate::typechecker::common::DependencyGroup;
use crate::typechecker::common::FunctionTypeInfo;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use crate::typechecker::walker::walk_expr;
use crate::typechecker::walker::Visitor;
use crate::util::format_list;
use std::collections::BTreeMap;

struct TypeVarCreator<'a> {
    expr_processor: &'a mut ExprProcessor,
}

impl<'a> TypeVarCreator<'a> {
    fn new(expr_processor: &'a mut ExprProcessor) -> TypeVarCreator<'a> {
        TypeVarCreator {
            expr_processor: expr_processor,
        }
    }
}

impl<'a> Visitor for TypeVarCreator<'a> {
    fn visit(&mut self, expr_id: ExprId, _: &Expr) {
        self.expr_processor.create_type_var_for_expr(expr_id);
    }
}

struct Unifier<'a> {
    expr_processor: &'a mut ExprProcessor,
    program: &'a Program,
    errors: &'a mut Vec<TypecheckError>,
    group: &'a DependencyGroup,
}

impl<'a> Unifier<'a> {
    fn new(
        expr_processor: &'a mut ExprProcessor,
        program: &'a Program,
        errors: &'a mut Vec<TypecheckError>,
        group: &'a DependencyGroup,
    ) -> Unifier<'a> {
        Unifier {
            expr_processor: expr_processor,
            program: program,
            errors: errors,
            group: group,
        }
    }
}

impl<'a> Unifier<'a> {
    fn get_function_type_var(&mut self, function_id: &FunctionId) -> TypeVariable {
        let type_info = self
            .expr_processor
            .function_type_info_map
            .get(function_id)
            .expect("Type info not found");
        if self.group.functions.contains(function_id) {
            return type_info.function_type;
        }
        self.expr_processor
            .type_store
            .clone_type_var(type_info.function_type)
    }

    fn check_literal(&mut self, expr_id: ExprId, ty: Type) {
        let literal_var = self.expr_processor.type_store.add_type(ty);
        let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
        let location = self.program.get_expr_location(&expr_id);
        self.expr_processor
            .unify_variables(&var, &literal_var, location, location, self.errors);
    }
}

impl<'a> Visitor for Unifier<'a> {
    fn visit(&mut self, expr_id: ExprId, expr: &Expr) {
        match expr {
            Expr::IntegerLiteral(_) => self.check_literal(expr_id, Type::Int),
            Expr::StringLiteral(_) => self.check_literal(expr_id, Type::String),
            Expr::BoolLiteral(_) => self.check_literal(expr_id, Type::Bool),
            Expr::FloatLiteral(_) => self.check_literal(expr_id, Type::Float),
            Expr::If(cond, true_branch, false_branch) => {
                let bool_var = self.expr_processor.type_store.add_type(Type::Bool);
                let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                let cond_var = self.expr_processor.lookup_type_var_for_expr(cond);
                let cond_location = self.program.get_expr_location(cond);
                let true_var = self.expr_processor.lookup_type_var_for_expr(true_branch);
                let true_location = self.program.get_expr_location(true_branch);
                let false_var = self.expr_processor.lookup_type_var_for_expr(false_branch);
                let false_location = self.program.get_expr_location(false_branch);
                self.expr_processor.unify_variables(
                    &bool_var,
                    &cond_var,
                    cond_location,
                    cond_location,
                    self.errors,
                );
                self.expr_processor.unify_variables(
                    &true_var,
                    &false_var,
                    true_location,
                    false_location,
                    self.errors,
                );
                self.expr_processor.unify_variables(
                    &true_var,
                    &var,
                    location,
                    location,
                    self.errors,
                );
            }
            Expr::StaticFunctionCall(function_id, args) => {
                let orig_function_type_var = self.get_function_type_var(function_id);
                let mut function_type_var = orig_function_type_var;
                let orig_arg_vars: Vec<_> = args
                    .iter()
                    .map(|arg| self.expr_processor.lookup_type_var_for_expr(arg))
                    .collect();
                let mut arg_vars = orig_arg_vars.clone();
                let mut failed = false;
                while !arg_vars.is_empty() {
                    if let Type::Function(func_type) =
                        self.expr_processor.type_store.get_type(&function_type_var)
                    {
                        if !self
                            .expr_processor
                            .type_store
                            .unify(&func_type.from, arg_vars.first().unwrap())
                        {
                            failed = true;
                            break;
                        } else {
                            function_type_var = func_type.to;
                            arg_vars.remove(0);
                        }
                    } else {
                        failed = true;
                        break;
                    }
                }
                let expr_var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                if failed {
                    let function_type_string = self
                        .expr_processor
                        .type_store
                        .get_resolved_type_string(&orig_function_type_var);
                    let arg_type_strings: Vec<_> = orig_arg_vars
                        .iter()
                        .map(|arg_var| {
                            self.expr_processor
                                .type_store
                                .get_resolved_type_string(arg_var)
                        })
                        .collect();
                    let arguments = format_list(&arg_type_strings[..]);
                    let err = TypecheckError::FunctionArgumentMismatch(
                        location,
                        arguments,
                        function_type_string,
                    );
                    self.errors.push(err);
                } else {
                    self.expr_processor.unify_variables(
                        &expr_var,
                        &function_type_var,
                        location,
                        location,
                        self.errors,
                    );
                }
            }
            Expr::DynamicFunctionCall(func_expr, args) => {
                let mut gen_args = Vec::new();
                let (gen_func, gen_result) = create_general_function_type(
                    args.len(),
                    &mut gen_args,
                    &mut self.expr_processor.type_store,
                );
                let mut failed = false;
                let func_expr_var = self.expr_processor.lookup_type_var_for_expr(func_expr);
                let arg_vars: Vec<_> = args
                    .iter()
                    .map(|arg| self.expr_processor.lookup_type_var_for_expr(arg))
                    .collect();
                if !self
                    .expr_processor
                    .type_store
                    .unify(&func_expr_var, &gen_func)
                {
                    failed = true;
                } else {
                    for (arg, gen_arg) in arg_vars.iter().zip(gen_args.iter()) {
                        if !self.expr_processor.type_store.unify(arg, gen_arg) {
                            failed = true;
                            break;
                        }
                    }
                }
                let expr_var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                if failed {
                    let function_type_string = self
                        .expr_processor
                        .type_store
                        .get_resolved_type_string(&gen_func);
                    let arg_type_strings: Vec<_> = arg_vars
                        .iter()
                        .map(|arg_var| {
                            self.expr_processor
                                .type_store
                                .get_resolved_type_string(arg_var)
                        })
                        .collect();
                    let arguments = format_list(&arg_type_strings[..]);
                    let err = TypecheckError::FunctionArgumentMismatch(
                        location,
                        arguments,
                        function_type_string,
                    );
                    self.errors.push(err);
                } else {
                    self.expr_processor.unify_variables(
                        &expr_var,
                        &gen_result,
                        location,
                        location,
                        self.errors,
                    );
                }
            }
            Expr::ArgRef(arg_ref) => {
                let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                let func = self.program.get_function(&arg_ref.id);
                let index = if arg_ref.captured {
                    arg_ref.index
                } else {
                    func.implicit_arg_count + arg_ref.index
                };
                let type_info = self
                    .expr_processor
                    .function_type_info_map
                    .get(&arg_ref.id)
                    .expect("Type info not found");
                let arg_var = type_info.args[index];
                self.expr_processor.unify_variables(
                    &var,
                    &arg_var,
                    location,
                    location,
                    self.errors,
                );
            }
            Expr::Do(items) => {
                let do_var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let do_location = self.program.get_expr_location(&expr_id);
                let last_item = items[items.len() - 1];
                let last_item_var = self.expr_processor.lookup_type_var_for_expr(&last_item);
                self.expr_processor.unify_variables(
                    &do_var,
                    &last_item_var,
                    do_location,
                    do_location,
                    self.errors,
                );
            }
            Expr::Tuple(items) => {
                let vars: Vec<_> = items
                    .iter()
                    .map(|i| self.expr_processor.lookup_type_var_for_expr(i))
                    .collect();
                let tuple_ty = Type::Tuple(vars);
                let tuple_var = self.expr_processor.type_store.add_type(tuple_ty);
                let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                self.expr_processor.unify_variables(
                    &tuple_var,
                    &var,
                    location,
                    location,
                    self.errors,
                );
            }
            Expr::TupleFieldAccess(index, tuple_expr) => {
                let tuple_var = self.expr_processor.lookup_type_var_for_expr(tuple_expr);
                let tuple_ty = self.expr_processor.type_store.get_type(&tuple_var);
                let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                if let Type::Tuple(items) = tuple_ty {
                    if items.len() > *index {
                        self.expr_processor.unify_variables(
                            &items[*index],
                            &var,
                            location,
                            location,
                            self.errors,
                        );
                        return;
                    }
                }
                let expected_type = format!("<tuple with at least {} item(s)>", index + 1);
                let found_type = self
                    .expr_processor
                    .type_store
                    .get_resolved_type_string(&tuple_var);
                let err =
                    TypecheckError::TypeMismatch(location, location, expected_type, found_type);
                self.errors.push(err);
            }
            Expr::Bind(_, rhs) => {
                let rhs_var = self.expr_processor.lookup_type_var_for_expr(rhs);
                let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                self.expr_processor.unify_variables(
                    &rhs_var,
                    &var,
                    location,
                    location,
                    self.errors,
                );
            }
            Expr::ExprValue(expr_ref) => {
                let expr_ref_var = self.expr_processor.lookup_type_var_for_expr(expr_ref);
                let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                self.expr_processor.unify_variables(
                    &expr_ref_var,
                    &var,
                    location,
                    location,
                    self.errors,
                );
            }
            Expr::Formatter(fmt, args) => {
                let subs: Vec<_> = fmt.split("{}").collect();
                if subs.len() != args.len() + 1 {
                    let location = self.program.get_expr_location(&expr_id);
                    let err = TypecheckError::InvalidFormatString(location);
                    self.errors.push(err);
                }
            }
            _ => panic!("Unifier: processing {} is not implemented", expr),
        }
    }
}

pub struct ExprProcessor {
    type_store: TypeStore,
    expression_type_var_map: BTreeMap<ExprId, TypeVariable>,
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
            function_type_info_map: function_type_info_map,
        }
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

    pub fn process_dep_group(
        &mut self,
        program: &Program,
        group: &DependencyGroup,
        errors: &mut Vec<TypecheckError>,
    ) {
        for function in &group.functions {
            self.process_function(function, program, errors, group);
        }
    }

    pub fn process_function(
        &mut self,
        function_id: &FunctionId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
        group: &DependencyGroup,
    ) {
        let type_info = self
            .function_type_info_map
            .get(function_id)
            .expect("Function type info not found");
        let body = type_info.body.expect("body not found");
        let result_var = type_info.result;
        let mut type_var_creator = TypeVarCreator::new(self);
        walk_expr(&body, program, &mut type_var_creator);
        let mut unifier = Unifier::new(self, program, errors, group);
        walk_expr(&body, program, &mut unifier);
        let body_var = self.lookup_type_var_for_expr(&body);
        let body_location = program.get_expr_location(&body);
        self.unify_variables(&result_var, &body_var, body_location, body_location, errors);
    }

    #[allow(unused)]
    pub fn dump_expression_types(&self, program: &Program) {
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

    #[allow(unused)]
    pub fn dump_function_types(&self) {
        for (id, info) in &self.function_type_info_map {
            if info.body.is_none() {
                continue;
            }
            println!(
                "{}/{}: {}",
                id,
                info.displayed_name,
                self.type_store
                    .get_resolved_type_string(&info.function_type)
            );
        }
    }

    pub fn check_recursive_types(&self, errors: &mut Vec<TypecheckError>) {
        for (_, info) in &self.function_type_info_map {
            if self.type_store.is_recursive(info.function_type) {
                let err = TypecheckError::RecursiveType(info.location_id);
                errors.push(err);
            }
        }
    }

    fn unify_variables(
        &mut self,
        expected: &TypeVariable,
        found: &TypeVariable,
        expected_location: LocationId,
        found_location: LocationId,
        errors: &mut Vec<TypecheckError>,
    ) -> bool {
        if !self.type_store.unify(&expected, &found) {
            let expected_type = self.type_store.get_resolved_type_string(&expected);
            let found_type = self.type_store.get_resolved_type_string(&found);
            let err = TypecheckError::TypeMismatch(
                found_location,
                expected_location,
                expected_type,
                found_type,
            );
            errors.push(err);
            false
        } else {
            true
        }
    }
}
