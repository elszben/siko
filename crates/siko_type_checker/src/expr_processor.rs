use crate::common::create_general_function_type;
use crate::common::DependencyGroup;
use crate::common::FunctionTypeInfo;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::error::TypecheckError;
use crate::type_store::TypeStore;
use crate::type_variable::TypeVariable;
use crate::types::Type;
use crate::walker::walk_expr;
use crate::walker::Visitor;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::expr::FieldAccessInfo;
use siko_ir::function::FunctionId;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_ir::types::TypeDefId;
use siko_location_info::item::LocationId;
use siko_util::format_list;
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
    fn visit_expr(&mut self, expr_id: ExprId, _: &Expr) {
        self.expr_processor.create_type_var_for_expr(expr_id);
    }

    fn visit_pattern(&mut self, pattern_id: PatternId, _: &Pattern) {
        self.expr_processor.create_type_var_for_pattern(pattern_id);
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
        let mut context = self.expr_processor.type_store.create_clone_context(false);
        context.clone_var(type_info.function_type)
    }

    fn check_literal_expr(&mut self, expr_id: ExprId, ty: Type) {
        let literal_var = self.expr_processor.type_store.add_type(ty);
        let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
        let location = self.program.get_expr_location(&expr_id);
        self.expr_processor
            .unify_variables(&var, &literal_var, location, self.errors);
    }

    fn check_literal_pattern(&mut self, pattern_id: PatternId, ty: Type) {
        let literal_var = self.expr_processor.type_store.add_type(ty);
        let var = self.expr_processor.lookup_type_var_for_pattern(&pattern_id);
        let location = self.program.get_pattern_location(&pattern_id);
        self.expr_processor
            .unify_variables(&var, &literal_var, location, self.errors);
    }

    #[allow(unused)]
    fn print_type(&self, msg: &str, var: &TypeVariable) {
        let ty = self.expr_processor.type_store.get_resolved_type_string(var);
        println!("{}: {}", msg, ty);
    }

    fn get_record_type_info(&mut self, record_id: &TypeDefId) -> RecordTypeInfo {
        let mut record_type_info = self
            .expr_processor
            .record_type_info_map
            .get(record_id)
            .expect("record tyoe info not found")
            .clone();
        let mut clone_context = self.expr_processor.type_store.create_clone_context(false);
        record_type_info.record_type = clone_context.clone_var(record_type_info.record_type);
        for field_type_var in &mut record_type_info.field_types {
            *field_type_var = clone_context.clone_var(*field_type_var);
        }
        record_type_info
    }
}

impl<'a> Visitor for Unifier<'a> {
    fn visit_expr(&mut self, expr_id: ExprId, expr: &Expr) {
        match expr {
            Expr::IntegerLiteral(_) => self.check_literal_expr(expr_id, Type::Int),
            Expr::StringLiteral(_) => self.check_literal_expr(expr_id, Type::String),
            Expr::BoolLiteral(_) => self.check_literal_expr(expr_id, Type::Bool),
            Expr::FloatLiteral(_) => self.check_literal_expr(expr_id, Type::Float),
            Expr::If(cond, true_branch, false_branch) => {
                let bool_var = self.expr_processor.type_store.add_type(Type::Bool);
                let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                let cond_var = self.expr_processor.lookup_type_var_for_expr(cond);
                let cond_location = self.program.get_expr_location(cond);
                let true_var = self.expr_processor.lookup_type_var_for_expr(true_branch);
                let true_location = self.program.get_expr_location(true_branch);
                let false_var = self.expr_processor.lookup_type_var_for_expr(false_branch);
                self.expr_processor.unify_variables(
                    &bool_var,
                    &cond_var,
                    cond_location,
                    self.errors,
                );
                self.expr_processor.unify_variables(
                    &true_var,
                    &false_var,
                    true_location,
                    self.errors,
                );
                self.expr_processor
                    .unify_variables(&true_var, &var, location, self.errors);
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
                        let first_arg = arg_vars.first().unwrap();
                        if !self
                            .expr_processor
                            .type_store
                            .unify(&func_type.from, first_arg)
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
                        self.errors,
                    );
                }
            }
            Expr::ArgRef(arg_ref) => {
                let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                let func = self.program.functions.get(&arg_ref.id);
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
                self.expr_processor
                    .unify_variables(&var, &arg_var, location, self.errors);
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
                self.expr_processor
                    .unify_variables(&tuple_var, &var, location, self.errors);
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
                let err = TypecheckError::TypeMismatch(location, expected_type, found_type);
                self.errors.push(err);
            }
            Expr::Bind(pattern_id, rhs) => {
                let pattern_var = self.expr_processor.lookup_type_var_for_pattern(pattern_id);
                let rhs_var = self.expr_processor.lookup_type_var_for_expr(rhs);
                let pattern_location = self.program.get_pattern_location(pattern_id);
                self.expr_processor.unify_variables(
                    &pattern_var,
                    &rhs_var,
                    pattern_location,
                    self.errors,
                );
                let tuple_ty = Type::Tuple(vec![]);
                let tuple_var = self.expr_processor.type_store.add_type(tuple_ty);
                let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                self.expr_processor
                    .unify_variables(&var, &tuple_var, location, self.errors);
            }
            Expr::ExprValue(expr_ref, _) => {
                let expr_ref_var = self.expr_processor.lookup_type_var_for_expr(expr_ref);
                let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                self.expr_processor
                    .unify_variables(&expr_ref_var, &var, location, self.errors);
            }
            Expr::Formatter(fmt, args) => {
                let subs: Vec<_> = fmt.split("{}").collect();
                if subs.len() != args.len() + 1 {
                    let location = self.program.get_expr_location(&expr_id);
                    let err = TypecheckError::InvalidFormatString(location);
                    self.errors.push(err);
                }
            }
            Expr::FieldAccess(infos, record_expr) => {
                let mut possible_records = Vec::new();
                let mut all_records = Vec::new();
                let record_expr_var = self.expr_processor.lookup_type_var_for_expr(record_expr);
                let expr_var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                let mut matches: Vec<(RecordTypeInfo, FieldAccessInfo)> = Vec::new();
                for info in infos {
                    let test_record_type_info = self.get_record_type_info(&info.record_id);
                    let record_type_info = self.get_record_type_info(&info.record_id);
                    let record = self.program.typedefs.get(&info.record_id).get_record();
                    all_records.push(record.name.clone());
                    let test_record_expr_var = self
                        .expr_processor
                        .type_store
                        .clone_type_var_simple(record_expr_var);
                    if self
                        .expr_processor
                        .type_store
                        .unify(&test_record_expr_var, &test_record_type_info.record_type)
                    {
                        possible_records.push(record.name.clone());
                        matches.push((record_type_info, info.clone()));
                    }
                }
                match matches.len() {
                    0 => {
                        let expected_type = format!("{}", all_records.join(" or "));
                        let found_type = self
                            .expr_processor
                            .type_store
                            .get_resolved_type_string(&record_expr_var);
                        let err = TypecheckError::TypeMismatch(location, expected_type, found_type);
                        self.errors.push(err);
                    }
                    1 => {
                        let (record_type_info, field_info) = &matches[0];
                        let field_type_var = record_type_info.field_types[field_info.index];
                        self.expr_processor.unify_variables(
                            &record_expr_var,
                            &record_type_info.record_type,
                            location,
                            self.errors,
                        );
                        self.expr_processor.unify_variables(
                            &expr_var,
                            &field_type_var,
                            location,
                            self.errors,
                        );
                    }
                    _ => {
                        let err = TypecheckError::AmbiguousFieldAccess(location, possible_records);
                        self.errors.push(err);
                    }
                }
            }
            Expr::CaseOf(body, cases) => {
                let body_var = self.expr_processor.lookup_type_var_for_expr(&body);
                let expr_var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                for case in cases {
                    let pattern_location = self.program.get_pattern_location(&case.pattern_id);
                    let pattern_var = self
                        .expr_processor
                        .lookup_type_var_for_pattern(&case.pattern_id);
                    self.expr_processor.unify_variables(
                        &body_var,
                        &pattern_var,
                        pattern_location,
                        self.errors,
                    );
                    let case_body_var = self.expr_processor.lookup_type_var_for_expr(&case.body);
                    let case_body_location = self.program.get_expr_location(&case.body);
                    self.expr_processor.unify_variables(
                        &expr_var,
                        &case_body_var,
                        case_body_location,
                        self.errors,
                    );
                }
            }
            Expr::RecordInitialization(type_id, items) => {
                let record_type_info = self.get_record_type_info(type_id);
                let expr_var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                let location = self.program.get_expr_location(&expr_id);
                self.expr_processor.unify_variables(
                    &expr_var,
                    &record_type_info.record_type,
                    location,
                    self.errors,
                );
                for (index, item) in items.iter().enumerate() {
                    let field_type_var = record_type_info.field_types[index];
                    let field_expr_var =
                        self.expr_processor.lookup_type_var_for_expr(&item.expr_id);
                    let field_expr_location = self.program.get_expr_location(&item.expr_id);
                    self.expr_processor.unify_variables(
                        &field_type_var,
                        &field_expr_var,
                        field_expr_location,
                        self.errors,
                    );
                }
            }
            Expr::RecordUpdate(record_expr_id, record_updates) => {
                let location_id = self.program.get_expr_location(&expr_id);
                let record_expr_var = self.expr_processor.lookup_type_var_for_expr(record_expr_id);
                let record_expr_type = self.expr_processor.type_store.get_type(&record_expr_var);
                let real_record_type = if let Type::Named(_, id, _) = record_expr_type {
                    Some(id)
                } else {
                    None
                };
                let mut expected_records = Vec::new();
                let mut matching_update = None;
                for record_update in record_updates {
                    let record = self
                        .program
                        .typedefs
                        .get(&record_update.record_id)
                        .get_record();
                    expected_records.push(record.name.clone());
                    if let Some(id) = real_record_type {
                        if record_update.record_id == id {
                            matching_update = Some(record_update);
                        }
                    }
                }
                match matching_update {
                    Some(update) => {
                        let record_type_info = self.get_record_type_info(&update.record_id);
                        self.expr_processor.unify_variables(
                            &record_type_info.record_type,
                            &record_expr_var,
                            location_id,
                            self.errors,
                        );
                        for field_update in &update.items {
                            let field_var = record_type_info.field_types[field_update.index];
                            let field_expr_var = self
                                .expr_processor
                                .lookup_type_var_for_expr(&field_update.expr_id);
                            let field_expr_location =
                                self.program.get_expr_location(&field_update.expr_id);
                            self.expr_processor.unify_variables(
                                &field_var,
                                &field_expr_var,
                                field_expr_location,
                                self.errors,
                            );
                        }
                        let expr_var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                        let location = self.program.get_expr_location(&expr_id);
                        self.expr_processor.unify_variables(
                            &expr_var,
                            &record_type_info.record_type,
                            location,
                            self.errors,
                        );
                    }
                    None => {
                        let expected_type = format!("{}", expected_records.join(" or "));
                        let found_type = self
                            .expr_processor
                            .type_store
                            .get_resolved_type_string(&record_expr_var);
                        let err =
                            TypecheckError::TypeMismatch(location_id, expected_type, found_type);
                        self.errors.push(err);
                    }
                }
            }
            Expr::ClassFunctionCall(..) => unimplemented!(),
        }
    }

    fn visit_pattern(&mut self, pattern_id: PatternId, pattern: &Pattern) {
        match pattern {
            Pattern::Binding(_) => {}
            Pattern::Tuple(items) => {
                let vars: Vec<_> = items
                    .iter()
                    .map(|i| self.expr_processor.lookup_type_var_for_pattern(i))
                    .collect();
                let tuple_ty = Type::Tuple(vars);
                let tuple_var = self.expr_processor.type_store.add_type(tuple_ty);
                let var = self.expr_processor.lookup_type_var_for_pattern(&pattern_id);
                let location = self.program.get_pattern_location(&pattern_id);
                self.expr_processor
                    .unify_variables(&tuple_var, &var, location, self.errors);
            }
            Pattern::Record(typedef_id, items) => {
                let record_type_info = self.get_record_type_info(typedef_id);
                let var = self.expr_processor.lookup_type_var_for_pattern(&pattern_id);
                let location = self.program.get_pattern_location(&pattern_id);
                self.expr_processor.unify_variables(
                    &record_type_info.record_type,
                    &var,
                    location,
                    self.errors,
                );
                if record_type_info.field_types.len() != items.len() {
                    let record = self.program.typedefs.get(typedef_id).get_record();
                    let err = TypecheckError::InvalidRecordPattern(
                        location,
                        record.name.clone(),
                        record_type_info.field_types.len(),
                        items.len(),
                    );
                    self.errors.push(err);
                } else {
                    for (index, item) in items.iter().enumerate() {
                        let item_var = self.expr_processor.lookup_type_var_for_pattern(item);
                        let field_var = record_type_info.field_types[index];
                        let location = self.program.get_pattern_location(item);
                        self.expr_processor.unify_variables(
                            &field_var,
                            &item_var,
                            location,
                            self.errors,
                        );
                    }
                }
            }
            Pattern::Variant(typedef_id, index, items) => {
                let variant_type_info = self
                    .expr_processor
                    .variant_type_info_map
                    .get(&(*typedef_id, *index))
                    .expect("Record type info not found");
                let mut clone_context = self.expr_processor.type_store.create_clone_context(false);
                let variant_var = clone_context.clone_var(variant_type_info.variant_type);
                let item_vars: Vec<_> = variant_type_info
                    .item_types
                    .iter()
                    .map(|v| clone_context.clone_var(*v))
                    .collect();
                let var = self.expr_processor.lookup_type_var_for_pattern(&pattern_id);
                let location = self.program.get_pattern_location(&pattern_id);
                self.expr_processor
                    .unify_variables(&variant_var, &var, location, self.errors);
                if item_vars.len() != items.len() {
                    let adt = self.program.typedefs.get(typedef_id).get_adt();
                    let variant = &adt.variants[*index];
                    let err = TypecheckError::InvalidVariantPattern(
                        location,
                        variant.name.clone(),
                        item_vars.len(),
                        items.len(),
                    );
                    self.errors.push(err);
                } else {
                    for (index, item) in items.iter().enumerate() {
                        let item_var = self.expr_processor.lookup_type_var_for_pattern(item);
                        let variant_item_var = item_vars[index];
                        let location = self.program.get_pattern_location(item);
                        self.expr_processor.unify_variables(
                            &variant_item_var,
                            &item_var,
                            location,
                            self.errors,
                        );
                    }
                }
            }
            Pattern::Guarded(_, guard_expr_id) => {
                let bool_var = self.expr_processor.type_store.add_type(Type::Bool);
                let guard_var = self.expr_processor.lookup_type_var_for_expr(guard_expr_id);
                let location = self.program.get_expr_location(guard_expr_id);
                self.expr_processor
                    .unify_variables(&bool_var, &guard_var, location, self.errors);
            }
            Pattern::Wildcard => {}
            Pattern::IntegerLiteral(_) => {
                self.check_literal_pattern(pattern_id, Type::Int);
            }
            Pattern::FloatLiteral(_) => {
                self.check_literal_pattern(pattern_id, Type::Float);
            }
            Pattern::StringLiteral(_) => {
                self.check_literal_pattern(pattern_id, Type::String);
            }
            Pattern::BoolLiteral(_) => {
                self.check_literal_pattern(pattern_id, Type::Bool);
            }
        }
    }
}

pub struct ExprProcessor {
    type_store: TypeStore,
    expression_type_var_map: BTreeMap<ExprId, TypeVariable>,
    pattern_type_var_map: BTreeMap<PatternId, TypeVariable>,
    function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    record_type_info_map: BTreeMap<TypeDefId, RecordTypeInfo>,
    variant_type_info_map: BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
}

impl ExprProcessor {
    pub fn new(
        type_store: TypeStore,
        function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
        record_type_info_map: BTreeMap<TypeDefId, RecordTypeInfo>,
        variant_type_info_map: BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    ) -> ExprProcessor {
        ExprProcessor {
            type_store: type_store,
            expression_type_var_map: BTreeMap::new(),
            pattern_type_var_map: BTreeMap::new(),
            function_type_info_map: function_type_info_map,
            record_type_info_map: record_type_info_map,
            variant_type_info_map: variant_type_info_map,
        }
    }

    fn create_type_var_for_expr(&mut self, expr_id: ExprId) -> TypeVariable {
        let var = self.type_store.get_new_type_var();
        self.expression_type_var_map.insert(expr_id, var);
        var
    }

    fn create_type_var_for_pattern(&mut self, pattern_id: PatternId) -> TypeVariable {
        let var = self.type_store.get_new_type_var();
        self.pattern_type_var_map.insert(pattern_id, var);
        var
    }

    pub fn lookup_type_var_for_expr(&self, expr_id: &ExprId) -> TypeVariable {
        *self
            .expression_type_var_map
            .get(expr_id)
            .expect("Type var for expr not found")
    }

    pub fn lookup_type_var_for_pattern(&self, pattern_id: &PatternId) -> TypeVariable {
        *self
            .pattern_type_var_map
            .get(pattern_id)
            .expect("Type var for pattern not found")
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
        self.unify_variables(&result_var, &body_var, body_location, errors);
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
                // continue;
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
        location: LocationId,
        errors: &mut Vec<TypecheckError>,
    ) -> bool {
        if !self.type_store.unify(&expected, &found) {
            let expected_type = self.type_store.get_resolved_type_string(&expected);
            let found_type = self.type_store.get_resolved_type_string(&found);
            let err = TypecheckError::TypeMismatch(location, expected_type, found_type);
            errors.push(err);
            false
        } else {
            true
        }
    }
}
