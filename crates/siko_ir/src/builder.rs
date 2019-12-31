use crate::data::Record;
use crate::data_type_info::RecordTypeInfo;
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
use siko_constants::FALSE_NAME;
use siko_constants::TRUE_NAME;
use siko_location_info::item::ItemInfo;
use siko_location_info::location_id::LocationId;

pub struct Builder<'a> {
    program: &'a mut Program,
}

impl<'a> Builder<'a> {
    pub fn new(program: &'a mut Program) -> Builder<'a> {
        Builder { program: program }
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
}
