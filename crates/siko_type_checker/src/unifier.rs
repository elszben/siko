use crate::common::create_general_function_type;
use crate::common::DependencyGroup;
use crate::common::RecordTypeInfo;
use crate::error::TypecheckError;
use crate::expr_processor::ExprProcessor;
use crate::type_processor::process_type_signature;
use crate::type_variable::TypeVariable;
use crate::types::Type;
use siko_constants::BOOL_NAME;
use siko_constants::FLOAT_NAME;
use siko_constants::INT_NAME;
use siko_constants::LIST_NAME;
use siko_constants::PRELUDE_NAME;
use siko_constants::STRING_NAME;
use siko_ir::class::ClassId;
use siko_ir::class::ClassMemberId;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::expr::FieldAccessInfo;
use siko_ir::function::FunctionId;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_ir::types::TypeDefId;
use siko_ir::walker::Visitor;
use siko_util::format_list;
use std::collections::BTreeMap;

pub struct Unifier<'a, 'b> {
    expr_processor: &'a mut ExprProcessor<'b>,
    errors: &'a mut Vec<TypecheckError>,
    group: &'a DependencyGroup,
    arg_map: BTreeMap<usize, TypeVariable>,
}

impl<'a, 'b: 'a> Unifier<'a, 'b> {
    pub fn new(
        expr_processor: &'a mut ExprProcessor<'b>,
        errors: &'a mut Vec<TypecheckError>,
        group: &'a DependencyGroup,
        arg_map: BTreeMap<usize, TypeVariable>,
    ) -> Unifier<'a, 'b> {
        Unifier {
            expr_processor: expr_processor,
            errors: errors,
            group: group,
            arg_map: arg_map,
        }
    }
}

impl<'a, 'b> Unifier<'a, 'b> {
    fn get_function_type_var(&mut self, function_id: &FunctionId) -> TypeVariable {
        let type_info = self
            .expr_processor
            .function_type_info_map
            .get(function_id)
            .expect("Type info not found");
        if self.group.functions.contains(function_id) {
            return type_info.function_type;
        }
        let mut context = self.expr_processor.type_store.create_clone_context();
        context.clone_var(type_info.function_type)
    }

    fn get_class_member_type_var(&mut self, class_member_id: &ClassMemberId) -> TypeVariable {
        let type_info = self
            .expr_processor
            .class_member_type_info_map
            .get(class_member_id)
            .expect("Type info not found");
        let mut context = self.expr_processor.type_store.create_clone_context();
        context.clone_var(type_info.member_type_var)
    }

    fn check_literal_expr(&mut self, expr_id: ExprId, literal_var: TypeVariable) {
        let var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
        let location = self.expr_processor.program.exprs.get(&expr_id).location_id;
        self.expr_processor
            .unify_variables(&var, &literal_var, location, self.errors);
    }

    fn check_literal_pattern(&mut self, pattern_id: PatternId, literal_var: TypeVariable) {
        let var = self.expr_processor.lookup_type_var_for_pattern(&pattern_id);
        let location = self
            .expr_processor
            .program
            .patterns
            .get(&pattern_id)
            .location_id;
        self.expr_processor
            .unify_variables(&var, &literal_var, location, self.errors);
    }

    #[allow(unused)]
    fn print_type(&self, msg: &str, var: &TypeVariable) {
        let ty = self.expr_processor.type_store.get_resolved_type_string(var);
        println!("{}: {}", msg, ty);
    }

    fn get_type_string(&self, var: &TypeVariable) -> String {
        self.expr_processor.type_store.get_resolved_type_string(var)
    }

    fn get_record_type_info(&mut self, record_id: &TypeDefId) -> RecordTypeInfo {
        let mut record_type_info = self
            .expr_processor
            .record_type_info_map
            .get(record_id)
            .expect("record tyoe info not found")
            .clone();
        let mut clone_context = self.expr_processor.type_store.create_clone_context();
        record_type_info.record_type = clone_context.clone_var(record_type_info.record_type);
        for field_type_var in &mut record_type_info.field_types {
            *field_type_var = clone_context.clone_var(*field_type_var);
        }
        record_type_info
    }

    fn match_patterns(&mut self, first: &PatternId, second: &PatternId) {
        let first_pattern_var = self.expr_processor.lookup_type_var_for_pattern(first);
        let second_pattern_var = self.expr_processor.lookup_type_var_for_pattern(second);
        let location = self.expr_processor.program.patterns.get(first).location_id;
        self.expr_processor.unify_variables(
            &first_pattern_var,
            &second_pattern_var,
            location,
            self.errors,
        );
    }

    fn match_pattern_with(&mut self, pattern: &PatternId, var: &TypeVariable) {
        let pattern_var = self.expr_processor.lookup_type_var_for_pattern(pattern);
        let location = self
            .expr_processor
            .program
            .patterns
            .get(pattern)
            .location_id;
        self.expr_processor
            .unify_variables(&var, &pattern_var, location, self.errors);
    }

    fn match_expr_with(&mut self, expr: &ExprId, var: &TypeVariable) -> bool {
        let expr_var = self.expr_processor.lookup_type_var_for_expr(expr);
        let location = self.expr_processor.program.exprs.get(expr).location_id;
        self.expr_processor
            .unify_variables(&var, &expr_var, location, self.errors)
    }

    fn get_builtin_type(&mut self, typedef_id: TypeDefId, args: Vec<TypeVariable>) -> TypeVariable {
        let name = self
            .get_program()
            .typedefs
            .get(&typedef_id)
            .get_record()
            .name
            .clone();
        let ty = Type::Named(name, typedef_id, args);
        let var = self.expr_processor.type_store.add_type(ty);
        var
    }

    fn get_type_class_var(&mut self, class_id: ClassId) -> TypeVariable {
        let type_arg = self.expr_processor.type_store.get_unique_type_arg();
        let ty = Type::TypeArgument(type_arg, vec![class_id]);
        let var = self.expr_processor.type_store.add_type(ty);
        var
    }

    fn get_show_type(&mut self) -> TypeVariable {
        let class_id = self
            .expr_processor
            .program
            .class_names
            .get("Show")
            .expect("Show not found")
            .clone();
        self.get_type_class_var(class_id)
    }

    fn get_bool_type(&mut self) -> TypeVariable {
        self.get_builtin_type(
            self.get_program().get_named_type(PRELUDE_NAME, BOOL_NAME),
            vec![],
        )
    }

    fn get_int_type(&mut self) -> TypeVariable {
        self.get_builtin_type(
            self.get_program().get_named_type("Data.Int", INT_NAME),
            vec![],
        )
    }

    fn get_string_type(&mut self) -> TypeVariable {
        self.get_builtin_type(
            self.get_program().get_named_type(PRELUDE_NAME, STRING_NAME),
            vec![],
        )
    }

    fn get_float_type(&mut self) -> TypeVariable {
        self.get_builtin_type(
            self.get_program().get_named_type("Data.Float", FLOAT_NAME),
            vec![],
        )
    }

    fn get_list_type(&mut self, type_var: TypeVariable) -> TypeVariable {
        self.get_builtin_type(
            self.get_program().get_named_type(PRELUDE_NAME, LIST_NAME),
            vec![type_var],
        )
    }

    fn check_function_call(
        &mut self,
        orig_func_var: TypeVariable,
        expr_id: ExprId,
        args: &Vec<ExprId>,
    ) {
        let mut gen_args = Vec::new();
        let (gen_func, gen_result) = create_general_function_type(
            args.len(),
            &mut gen_args,
            &mut self.expr_processor.type_store,
        );
        let arg_vars: Vec<_> = args
            .iter()
            .map(|arg| self.expr_processor.lookup_type_var_for_expr(arg))
            .collect();
        let location = self.expr_processor.program.exprs.get(&expr_id).location_id;
        if self
            .expr_processor
            .unify_variables(&gen_func, &orig_func_var, location, self.errors)
        {
            let mut failed = false;
            for (arg, gen_arg) in arg_vars.iter().zip(gen_args.iter()) {
                if !self.expr_processor.type_store.unify(arg, gen_arg) {
                    failed = true;
                    break;
                }
            }
            if failed {
                let function_type_string = self.get_type_string(&gen_func);
                let arg_type_strings: Vec<_> = arg_vars
                    .iter()
                    .map(|arg_var| self.get_type_string(arg_var))
                    .collect();
                let arguments = format_list(&arg_type_strings[..]);
                let err = TypecheckError::FunctionArgumentMismatch(
                    location,
                    arguments,
                    function_type_string,
                );
                self.errors.push(err);
            } else {
                self.match_expr_with(&expr_id, &gen_result);
            }
        }
    }
}

impl<'a, 'b> Visitor for Unifier<'a, 'b> {
    fn get_program(&self) -> &Program {
        &self.expr_processor.program
    }

    fn visit_expr(&mut self, expr_id: ExprId, expr: &Expr) {
        match expr {
            Expr::IntegerLiteral(_) => {
                let var = self.get_int_type();
                self.check_literal_expr(expr_id, var)
            }
            Expr::StringLiteral(_) => {
                let var = self.get_string_type();
                self.check_literal_expr(expr_id, var)
            }
            Expr::BoolLiteral(_) => {
                let var = self.get_bool_type();
                self.check_literal_expr(expr_id, var)
            }
            Expr::FloatLiteral(_) => {
                let var = self.get_float_type();
                self.check_literal_expr(expr_id, var)
            }
            Expr::If(cond, true_branch, false_branch) => {
                let bool_var = self.get_bool_type();
                let false_var = self.expr_processor.lookup_type_var_for_expr(false_branch);
                self.match_expr_with(cond, &bool_var);
                self.match_expr_with(true_branch, &false_var);
                self.match_expr_with(&expr_id, &false_var);
            }
            Expr::StaticFunctionCall(function_id, args) => {
                let orig_function_type_var = self.get_function_type_var(function_id);
                self.check_function_call(orig_function_type_var, expr_id, args);
            }
            Expr::DynamicFunctionCall(func_expr, args) => {
                let func_var = self.expr_processor.lookup_type_var_for_expr(func_expr);
                self.check_function_call(func_var, expr_id, args);
            }
            Expr::ArgRef(arg_ref) => {
                let func = self.expr_processor.program.functions.get(&arg_ref.id);
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
                self.match_expr_with(&expr_id, &arg_var);
            }
            Expr::Do(items) => {
                let last_item = items[items.len() - 1];
                let last_item_var = self.expr_processor.lookup_type_var_for_expr(&last_item);
                self.match_expr_with(&expr_id, &last_item_var);
            }
            Expr::Tuple(items) => {
                let vars: Vec<_> = items
                    .iter()
                    .map(|i| self.expr_processor.lookup_type_var_for_expr(i))
                    .collect();
                let tuple_ty = Type::Tuple(vars);
                let tuple_var = self.expr_processor.type_store.add_type(tuple_ty);
                self.match_expr_with(&expr_id, &tuple_var);
            }
            Expr::List(items) => {
                let vars: Vec<_> = items
                    .iter()
                    .map(|i| self.expr_processor.lookup_type_var_for_expr(i))
                    .collect();
                let item_var = if vars.is_empty() {
                    self.expr_processor.type_store.get_new_type_var()
                } else {
                    let first = vars[0];
                    for index in 1..items.len() {
                        self.match_expr_with(&items[index], &first);
                    }
                    vars[0]
                };
                let list_type_var = self.get_list_type(item_var);
                self.match_expr_with(&expr_id, &list_type_var);
            }
            Expr::TupleFieldAccess(index, tuple_expr) => {
                let tuple_var = self.expr_processor.lookup_type_var_for_expr(tuple_expr);
                let tuple_ty = self.expr_processor.type_store.get_type(&tuple_var);
                let location = self.expr_processor.program.exprs.get(&expr_id).location_id;
                if let Type::Tuple(items) = tuple_ty {
                    if items.len() > *index {
                        self.match_expr_with(&expr_id, &items[*index]);
                        return;
                    }
                }
                let expected_type = format!("<tuple with at least {} item(s)>", index + 1);
                let found_type = self.get_type_string(&tuple_var);
                let err = TypecheckError::TypeMismatch(location, expected_type, found_type);
                self.errors.push(err);
            }
            Expr::Bind(pattern_id, rhs) => {
                let rhs_var = self.expr_processor.lookup_type_var_for_expr(rhs);
                self.match_pattern_with(pattern_id, &rhs_var);
                let tuple_ty = Type::Tuple(vec![]);
                let tuple_var = self.expr_processor.type_store.add_type(tuple_ty);
                self.match_expr_with(&expr_id, &tuple_var);
            }
            Expr::ExprValue(_, pattern_id) => {
                let expr_var = self.expr_processor.lookup_type_var_for_expr(&expr_id);
                self.match_pattern_with(pattern_id, &expr_var);
            }
            Expr::Formatter(fmt, args) => {
                let subs: Vec<_> = fmt.split("{}").collect();
                if subs.len() != args.len() + 1 {
                    let location = self.expr_processor.program.exprs.get(&expr_id).location_id;
                    let err = TypecheckError::InvalidFormatString(location);
                    self.errors.push(err);
                }
                for arg in args {
                    let var = self.get_show_type();
                    self.match_expr_with(arg, &var);
                }
            }
            Expr::FieldAccess(infos, record_expr) => {
                let mut possible_records = Vec::new();
                let mut all_records = Vec::new();
                let record_expr_var = self.expr_processor.lookup_type_var_for_expr(record_expr);
                let location = self.expr_processor.program.exprs.get(&expr_id).location_id;
                let mut matches: Vec<(RecordTypeInfo, FieldAccessInfo)> = Vec::new();
                for info in infos {
                    let test_record_type_info = self.get_record_type_info(&info.record_id);
                    let record_type_info = self.get_record_type_info(&info.record_id);
                    let record = self
                        .expr_processor
                        .program
                        .typedefs
                        .get(&info.record_id)
                        .get_record();
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
                        let found_type = self.get_type_string(&record_expr_var);
                        let err = TypecheckError::TypeMismatch(location, expected_type, found_type);
                        self.errors.push(err);
                    }
                    1 => {
                        let (record_type_info, field_info) = &matches[0];
                        let field_type_var = record_type_info.field_types[field_info.index];
                        self.match_expr_with(record_expr, &record_type_info.record_type);
                        self.match_expr_with(&expr_id, &field_type_var);
                    }
                    _ => {
                        let err = TypecheckError::AmbiguousFieldAccess(location, possible_records);
                        self.errors.push(err);
                    }
                }
            }
            Expr::CaseOf(body, cases) => {
                let body_var = self.expr_processor.lookup_type_var_for_expr(&body);
                for case in cases {
                    self.match_pattern_with(&case.pattern_id, &body_var);
                    let case_body_var = self.expr_processor.lookup_type_var_for_expr(&case.body);
                    self.match_expr_with(&expr_id, &case_body_var);
                }
            }
            Expr::RecordInitialization(type_id, items) => {
                let record_type_info = self.get_record_type_info(type_id);
                self.match_expr_with(&expr_id, &record_type_info.record_type);
                for (index, item) in items.iter().enumerate() {
                    let field_type_var = record_type_info.field_types[index];
                    self.match_expr_with(&item.expr_id, &field_type_var);
                }
            }
            Expr::RecordUpdate(record_expr_id, record_updates) => {
                let location_id = self.expr_processor.program.exprs.get(&expr_id).location_id;
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
                        .expr_processor
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
                        self.match_expr_with(record_expr_id, &record_type_info.record_type);
                        for field_update in &update.items {
                            let field_var = record_type_info.field_types[field_update.index];
                            self.match_expr_with(&field_update.expr_id, &field_var);
                        }
                        self.match_expr_with(&expr_id, &record_type_info.record_type);
                    }
                    None => {
                        let expected_type = format!("{}", expected_records.join(" or "));
                        let found_type = self.get_type_string(&record_expr_var);
                        let err =
                            TypecheckError::TypeMismatch(location_id, expected_type, found_type);
                        self.errors.push(err);
                    }
                }
            }
            Expr::ClassFunctionCall(class_member_id, args) => {
                let orig_function_type_var = self.get_class_member_type_var(class_member_id);
                self.check_function_call(orig_function_type_var, expr_id, args);
            }
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
                self.match_pattern_with(&pattern_id, &tuple_var);
            }
            Pattern::Record(typedef_id, items) => {
                let record_type_info = self.get_record_type_info(typedef_id);
                self.match_pattern_with(&pattern_id, &record_type_info.record_type);
                if record_type_info.field_types.len() != items.len() {
                    let location = self
                        .expr_processor
                        .program
                        .patterns
                        .get(&pattern_id)
                        .location_id;
                    let record = self
                        .expr_processor
                        .program
                        .typedefs
                        .get(typedef_id)
                        .get_record();
                    let err = TypecheckError::InvalidRecordPattern(
                        location,
                        record.name.clone(),
                        record_type_info.field_types.len(),
                        items.len(),
                    );
                    self.errors.push(err);
                } else {
                    for (index, item) in items.iter().enumerate() {
                        let field_var = record_type_info.field_types[index];
                        self.match_pattern_with(item, &field_var);
                    }
                }
            }
            Pattern::Variant(typedef_id, index, items) => {
                let variant_type_info = self
                    .expr_processor
                    .variant_type_info_map
                    .get(&(*typedef_id, *index))
                    .expect("Record type info not found");
                let mut clone_context = self.expr_processor.type_store.create_clone_context();
                let variant_var = clone_context.clone_var(variant_type_info.variant_type);
                let item_vars: Vec<_> = variant_type_info
                    .item_types
                    .iter()
                    .map(|v| clone_context.clone_var(*v))
                    .collect();
                self.match_pattern_with(&pattern_id, &variant_var);
                let location = self
                    .expr_processor
                    .program
                    .patterns
                    .get(&pattern_id)
                    .location_id;
                if item_vars.len() != items.len() {
                    let adt = self
                        .expr_processor
                        .program
                        .typedefs
                        .get(typedef_id)
                        .get_adt();
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
                        let variant_item_var = item_vars[index];
                        self.match_pattern_with(item, &variant_item_var);
                    }
                }
            }
            Pattern::Guarded(inner, guard_expr_id) => {
                self.match_patterns(inner, &pattern_id);
                let bool_var = self.get_bool_type();
                self.match_expr_with(guard_expr_id, &bool_var);
            }
            Pattern::Wildcard => {}
            Pattern::IntegerLiteral(_) => {
                let var = self.get_int_type();
                self.check_literal_pattern(pattern_id, var);
            }
            Pattern::FloatLiteral(_) => {
                let var = self.get_float_type();
                self.check_literal_pattern(pattern_id, var);
            }
            Pattern::StringLiteral(_) => {
                let var = self.get_string_type();
                self.check_literal_pattern(pattern_id, var);
            }
            Pattern::BoolLiteral(_) => {
                let var = self.get_bool_type();
                self.check_literal_pattern(pattern_id, var);
            }
            Pattern::Typed(inner, type_signature_id) => {
                self.match_patterns(inner, &pattern_id);
                let mut arg_map = self.arg_map.clone();
                let pattern_signature_type_var = process_type_signature(
                    &mut self.expr_processor.type_store,
                    type_signature_id,
                    self.expr_processor.program,
                    &mut arg_map,
                    &mut None,
                );
                self.match_pattern_with(inner, &pattern_signature_type_var);
            }
        }
    }
}
