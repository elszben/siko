use crate::class::ClassMemberId;
use crate::data::Adt;
use crate::data::Record;
use crate::data_type_info::AdtTypeInfo;
use crate::data_type_info::RecordTypeInfo;
use crate::expr::Case;
use crate::expr::Expr;
use crate::expr::ExprId;
use crate::expr::FunctionArgumentRef;
use crate::function::FunctionId;
use crate::pattern::Pattern;
use crate::pattern::PatternId;
use crate::program::Program;
use crate::types::Type;
use siko_constants::BOOL_MODULE_NAME;
use siko_constants::BOOL_TYPE_NAME;
use siko_constants::EQUAL_NAME;
use siko_constants::FALSE_NAME;
use siko_constants::GREATER_NAME;
use siko_constants::LESS_NAME;
use siko_constants::NONE_NAME;
use siko_constants::OPTION_MODULE_NAME;
use siko_constants::OPTION_TYPE_NAME;
use siko_constants::ORDERING_MODULE_NAME;
use siko_constants::ORDERING_TYPE_NAME;
use siko_constants::SOME_NAME;
use siko_constants::TRUE_NAME;
use siko_location_info::item::ItemInfo;
use siko_location_info::location_id::LocationId;
use siko_util::Counter;
use std::cmp::Ordering;

pub struct Builder<'a> {
    program: &'a mut Program,
    temp_var_counter: Counter,
}

impl<'a> Builder<'a> {
    pub fn new(program: &'a mut Program) -> Builder<'a> {
        Builder {
            program: program,
            temp_var_counter: Counter::new(),
        }
    }

    pub fn get_temp_var(&mut self) -> String {
        format!("$_irbuild_{}", self.temp_var_counter.next())
    }

    pub fn create_bool(&mut self, value: bool, location: LocationId) -> ExprId {
        let bool_ty = self.program.get_bool_type();
        let ctor = self.program.get_constructor_by_name(
            BOOL_MODULE_NAME,
            BOOL_TYPE_NAME,
            if value { TRUE_NAME } else { FALSE_NAME },
        );
        let expr = Expr::StaticFunctionCall(ctor, vec![]);
        self.add_expr(expr, location, bool_ty)
    }

    pub fn create_optional_ordering(
        &mut self,
        value: Option<Ordering>,
        location: LocationId,
    ) -> ExprId {
        let ordering_ty = self.program.get_ordering_type();
        let optional_ordering_ty = self.program.get_option_type(ordering_ty.clone());
        match value {
            Some(v) => {
                let name = match v {
                    Ordering::Equal => EQUAL_NAME,
                    Ordering::Less => LESS_NAME,
                    Ordering::Greater => GREATER_NAME,
                };
                let ctor = self.program.get_constructor_by_name(
                    ORDERING_MODULE_NAME,
                    ORDERING_TYPE_NAME,
                    name,
                );
                let expr = Expr::StaticFunctionCall(ctor, vec![]);
                let ordering_expr_id = self.add_expr(expr, location, ordering_ty);
                let ctor = self.program.get_constructor_by_name(
                    OPTION_MODULE_NAME,
                    OPTION_TYPE_NAME,
                    SOME_NAME,
                );
                let expr = Expr::StaticFunctionCall(ctor, vec![ordering_expr_id]);
                self.add_expr(expr, location, optional_ordering_ty)
            }
            None => {
                let ctor = self.program.get_constructor_by_name(
                    OPTION_MODULE_NAME,
                    OPTION_TYPE_NAME,
                    NONE_NAME,
                );
                let expr = Expr::StaticFunctionCall(ctor, vec![]);
                self.add_expr(expr, location, optional_ordering_ty)
            }
        }
    }

    pub fn add_arg_ref(
        &mut self,
        index: usize,
        function_id: FunctionId,
        location: LocationId,
        arg_ty: Type,
    ) -> ExprId {
        let arg_ref = FunctionArgumentRef::new(false, function_id, index);
        let arg_ref_expr = Expr::ArgRef(arg_ref);
        let arg_ref_expr_id = self.add_expr(arg_ref_expr, location, arg_ty);
        arg_ref_expr_id
    }

    pub fn add_record_pattern(
        &mut self,
        source_expr: ExprId,
        record: &Record,
        record_type_info: &RecordTypeInfo,
        location: LocationId,
    ) -> (ExprId, Vec<ExprId>) {
        let mut field_patterns = Vec::new();
        let mut values = Vec::new();
        for (index, (field_type, _)) in record_type_info.field_types.iter().enumerate() {
            let field = &record.fields[index];
            let field_pattern = Pattern::Binding(field.name.clone());
            let field_pattern_id = self.add_pattern(field_pattern, location, field_type.clone());
            field_patterns.push(field_pattern_id);
            let expr_value_expr = Expr::ExprValue(source_expr, field_pattern_id);
            let expr_value_expr_id = self.add_expr(expr_value_expr, location, field_type.clone());
            values.push(expr_value_expr_id);
        }
        let pattern = Pattern::Record(record.id, field_patterns);
        let pattern_id = self.add_pattern(pattern, location, record_type_info.record_type.clone());
        let bind_expr = Expr::Bind(pattern_id, source_expr);
        let bind_expr_id = self.add_expr(bind_expr, location, Type::Tuple(vec![]));
        (bind_expr_id, values)
    }

    pub fn add_bind_pattern(
        &mut self,
        source_expr: ExprId,
        ty: &Type,
        location: LocationId,
    ) -> (ExprId, ExprId) {
        let name = self.get_temp_var();
        let pattern = Pattern::Binding(name.clone());
        let pattern_id = self.add_pattern(pattern, location, ty.clone());
        let expr_value_expr = Expr::ExprValue(source_expr, pattern_id);
        let expr_value_expr_id = self.add_expr(expr_value_expr, location, ty.clone());
        let bind_expr = Expr::Bind(pattern_id, source_expr);
        let bind_expr_id = self.add_expr(bind_expr, location, Type::Tuple(vec![]));
        (bind_expr_id, expr_value_expr_id)
    }

    pub fn add_expr(&mut self, expr: Expr, location_id: LocationId, expr_ty: Type) -> ExprId {
        let id = self.program.exprs.get_id();
        self.program
            .exprs
            .add_item(id, ItemInfo::new(expr, location_id));
        self.program.expr_types.insert(id, expr_ty);
        id
    }

    pub fn add_pattern(
        &mut self,
        pattern: Pattern,
        location_id: LocationId,
        pattern_ty: Type,
    ) -> PatternId {
        let id = self.program.patterns.get_id();
        self.program
            .patterns
            .add_item(id, ItemInfo::new(pattern, location_id));
        self.program.pattern_types.insert(id, pattern_ty);
        id
    }

    pub fn generate_show_instance_member_for_record(
        &mut self,
        location: LocationId,
        function_id: FunctionId,
        record: &Record,
        record_type_info: RecordTypeInfo,
    ) -> (ExprId, Type) {
        let string_ty = self.program.get_string_type();
        let arg_ref_expr_id = self.add_arg_ref(
            0,
            function_id,
            location,
            record_type_info.record_type.clone(),
        );
        let (bind_expr_id, values) =
            self.add_record_pattern(arg_ref_expr_id, record, &record_type_info, location);
        let field_fmt_str_args: Vec<_> = std::iter::repeat("{{}}").take(values.len()).collect();
        let fmt_str = format!("{} {{ {} }}", record.name, field_fmt_str_args.join(", "));
        let fmt_expr = Expr::Formatter(fmt_str, values);
        let fmt_expr_id = self.add_expr(fmt_expr, location, string_ty.clone());
        let items = vec![bind_expr_id, fmt_expr_id];
        let body = self.add_expr(Expr::Do(items), location, string_ty.clone());
        let function_type = Type::Function(
            Box::new(record_type_info.record_type.clone()),
            Box::new(string_ty),
        );
        (body, function_type)
    }

    pub fn generate_show_instance_member_for_adt(
        &mut self,
        location: LocationId,
        function_id: FunctionId,
        adt: &Adt,
        adt_type_info: AdtTypeInfo,
    ) -> (ExprId, Type) {
        let string_ty = self.program.get_string_type();
        let arg_ref_expr_id =
            self.add_arg_ref(0, function_id, location, adt_type_info.adt_type.clone());
        let mut cases = Vec::new();
        for (index, variant) in adt_type_info.variant_types.iter().enumerate() {
            let mut item_patterns = Vec::new();
            let mut values = Vec::new();
            for (item_type, _) in &variant.item_types {
                let item_pattern = Pattern::Binding(self.get_temp_var());
                let item_pattern_id = self.add_pattern(item_pattern, location, item_type.clone());
                item_patterns.push(item_pattern_id);
                let expr_value_expr = Expr::ExprValue(arg_ref_expr_id, item_pattern_id);
                let expr_value_expr_id =
                    self.add_expr(expr_value_expr, location, item_type.clone());
                values.push(expr_value_expr_id);
            }
            let pattern = Pattern::Variant(adt.id, index, item_patterns);
            let pattern_id = self.add_pattern(pattern, location, adt_type_info.adt_type.clone());
            let item_fmt_str_args: Vec<_> = std::iter::repeat("{{}}").take(values.len()).collect();
            let fmt_str = format!("{} {} ", adt.name, item_fmt_str_args.join(", "));
            let fmt_expr = Expr::Formatter(fmt_str, values);
            let fmt_expr_id = self.add_expr(fmt_expr, location, string_ty.clone());
            let case = Case {
                pattern_id: pattern_id,
                body: fmt_expr_id,
            };
            cases.push(case);
        }
        let case_expr = Expr::CaseOf(arg_ref_expr_id, cases, Vec::new());
        let body = self.add_expr(case_expr, location, string_ty.clone());
        let function_type = Type::Function(
            Box::new(adt_type_info.adt_type.clone()),
            Box::new(string_ty),
        );
        (body, function_type)
    }

    pub fn generate_partialeq_instance_member_for_record(
        &mut self,
        location: LocationId,
        function_id: FunctionId,
        record: &Record,
        record_type_info: RecordTypeInfo,
        class_member_id: ClassMemberId,
    ) -> (ExprId, Type) {
        let bool_ty = self.program.get_bool_type();
        let arg_ref_expr_id_0 = self.add_arg_ref(
            0,
            function_id,
            location,
            record_type_info.record_type.clone(),
        );
        let arg_ref_expr_id_1 = self.add_arg_ref(
            1,
            function_id,
            location,
            record_type_info.record_type.clone(),
        );
        let (bind_expr_id_0, values_0) =
            self.add_record_pattern(arg_ref_expr_id_0, record, &record_type_info, location);
        let (bind_expr_id_1, values_1) =
            self.add_record_pattern(arg_ref_expr_id_1, record, &record_type_info, location);
        let mut true_branch = self.create_bool(true, location);
        for (value_0, value_1) in values_0.iter().zip(values_1.iter()) {
            let call_expr = Expr::ClassFunctionCall(class_member_id, vec![*value_0, *value_1]);
            let call_expr_id = self.add_expr(call_expr, location, bool_ty.clone());
            let false_branch = self.create_bool(false, location);
            let if_expr = Expr::If(call_expr_id, true_branch, false_branch);
            true_branch = self.add_expr(if_expr, location, bool_ty.clone());
        }
        let items = vec![bind_expr_id_0, bind_expr_id_1, true_branch];
        let body = self.add_expr(Expr::Do(items), location, bool_ty.clone());
        let function_type = Type::Function(
            Box::new(record_type_info.record_type.clone()),
            Box::new(bool_ty),
        );
        let function_type = Type::Function(
            Box::new(record_type_info.record_type.clone()),
            Box::new(function_type),
        );
        (body, function_type)
    }

    pub fn generate_partialeq_instance_member_for_adt(
        &mut self,
        location: LocationId,
        function_id: FunctionId,
        adt: &Adt,
        adt_type_info: AdtTypeInfo,
        class_member_id: ClassMemberId,
    ) -> (ExprId, Type) {
        let bool_ty = self.program.get_bool_type();
        let arg_ref_expr_id_0 =
            self.add_arg_ref(0, function_id, location, adt_type_info.adt_type.clone());
        let arg_ref_expr_id_1 =
            self.add_arg_ref(1, function_id, location, adt_type_info.adt_type.clone());
        let tuple_expr = Expr::Tuple(vec![arg_ref_expr_id_0, arg_ref_expr_id_1]);
        let tuple_ty = Type::Tuple(vec![
            adt_type_info.adt_type.clone(),
            adt_type_info.adt_type.clone(),
        ]);
        let tuple_expr_id = self.add_expr(tuple_expr, location, tuple_ty.clone());
        let mut cases = Vec::new();
        for (index0, variant0) in adt_type_info.variant_types.iter().enumerate() {
            for (index1, variant1) in adt_type_info.variant_types.iter().enumerate() {
                let mut item_patterns0 = Vec::new();
                let mut item_patterns1 = Vec::new();
                let mut values0 = Vec::new();
                let mut values1 = Vec::new();
                for (item_type, _) in &variant0.item_types {
                    let item_pattern = Pattern::Binding(self.get_temp_var());
                    let item_pattern_id =
                        self.add_pattern(item_pattern, location, item_type.clone());
                    item_patterns0.push(item_pattern_id);
                    let expr_value_expr = Expr::ExprValue(arg_ref_expr_id_0, item_pattern_id);
                    let expr_value_expr_id =
                        self.add_expr(expr_value_expr, location, item_type.clone());
                    values0.push(expr_value_expr_id);
                }
                for (item_type, _) in &variant1.item_types {
                    let item_pattern = Pattern::Binding(self.get_temp_var());
                    let item_pattern_id =
                        self.add_pattern(item_pattern, location, item_type.clone());
                    item_patterns1.push(item_pattern_id);
                    let expr_value_expr = Expr::ExprValue(arg_ref_expr_id_1, item_pattern_id);
                    let expr_value_expr_id =
                        self.add_expr(expr_value_expr, location, item_type.clone());
                    values1.push(expr_value_expr_id);
                }
                let pattern0 = Pattern::Variant(adt.id, index0, item_patterns0);
                let pattern0_id =
                    self.add_pattern(pattern0, location, adt_type_info.adt_type.clone());
                let pattern1 = Pattern::Variant(adt.id, index1, item_patterns1);
                let pattern1_id =
                    self.add_pattern(pattern1, location, adt_type_info.adt_type.clone());

                let tuple_pattern = Pattern::Tuple(vec![pattern0_id, pattern1_id]);
                let tuple_pattern_id = self.add_pattern(tuple_pattern, location, tuple_ty.clone());
                let mut true_branch = self.create_bool(true, location);
                for (value_0, value_1) in values0.iter().zip(values1.iter()) {
                    let call_expr =
                        Expr::ClassFunctionCall(class_member_id, vec![*value_0, *value_1]);
                    let call_expr_id = self.add_expr(call_expr, location, bool_ty.clone());
                    let false_branch = self.create_bool(false, location);
                    let if_expr = Expr::If(call_expr_id, true_branch, false_branch);
                    true_branch = self.add_expr(if_expr, location, bool_ty.clone());
                }
                let case = Case {
                    pattern_id: tuple_pattern_id,
                    body: true_branch,
                };
                cases.push(case);
            }
        }
        let case_expr = Expr::CaseOf(tuple_expr_id, cases, Vec::new());
        let body = self.add_expr(case_expr, location, bool_ty.clone());
        let function_type =
            Type::Function(Box::new(adt_type_info.adt_type.clone()), Box::new(bool_ty));
        let function_type = Type::Function(
            Box::new(adt_type_info.adt_type.clone()),
            Box::new(function_type),
        );
        (body, function_type)
    }

    pub fn generate_partialord_instance_member_for_record(
        &mut self,
        location: LocationId,
        function_id: FunctionId,
        record: &Record,
        record_type_info: RecordTypeInfo,
        class_member_id: ClassMemberId,
    ) -> (ExprId, Type) {
        let optional_ordering_ty = self
            .program
            .get_option_type(self.program.get_ordering_type());
        let partialeq_op_id = self.program.get_partialeq_op_id();
        let bool_ty = self.program.get_bool_type();
        let arg_ref_expr_id_0 = self.add_arg_ref(
            0,
            function_id,
            location,
            record_type_info.record_type.clone(),
        );
        let arg_ref_expr_id_1 = self.add_arg_ref(
            1,
            function_id,
            location,
            record_type_info.record_type.clone(),
        );
        let (bind_expr_id_0, values_0) =
            self.add_record_pattern(arg_ref_expr_id_0, record, &record_type_info, location);
        let (bind_expr_id_1, values_1) =
            self.add_record_pattern(arg_ref_expr_id_1, record, &record_type_info, location);
        let mut true_branch = self.create_optional_ordering(Some(Ordering::Equal), location);
        for (value_0, value_1) in values_0.iter().zip(values_1.iter()) {
            let call_expr = Expr::ClassFunctionCall(class_member_id, vec![*value_0, *value_1]);
            let call_expr_id = self.add_expr(call_expr, location, optional_ordering_ty.clone());
            let (bind, value_expr_id) =
                self.add_bind_pattern(call_expr_id, &optional_ordering_ty, location);
            let optional_equal_expr_id =
                self.create_optional_ordering(Some(Ordering::Equal), location);
            let partialeq_call_expr = Expr::ClassFunctionCall(
                partialeq_op_id,
                vec![value_expr_id, optional_equal_expr_id],
            );
            let partialeq_expr_id = self.add_expr(partialeq_call_expr, location, bool_ty.clone());
            let if_expr = Expr::If(partialeq_expr_id, true_branch, value_expr_id);
            let if_expr_id = self.add_expr(if_expr, location, optional_ordering_ty.clone());
            let inner_do_items = vec![bind, if_expr_id];
            true_branch = self.add_expr(
                Expr::Do(inner_do_items),
                location,
                optional_ordering_ty.clone(),
            );
        }
        let items = vec![bind_expr_id_0, bind_expr_id_1, true_branch];
        let body = self.add_expr(Expr::Do(items), location, optional_ordering_ty.clone());
        let function_type = Type::Function(
            Box::new(record_type_info.record_type.clone()),
            Box::new(optional_ordering_ty),
        );
        let function_type = Type::Function(
            Box::new(record_type_info.record_type.clone()),
            Box::new(function_type),
        );
        (body, function_type)
    }
}
