use crate::dependency_processor::DependencyGroup;
use crate::error::TypecheckError;
use crate::type_info_provider::TypeInfoProvider;
use crate::type_store::TypeStore;
use crate::types::Type;
use crate::util::create_general_function_type;
use crate::util::get_bool_type;
use crate::util::get_float_type;
use crate::util::get_int_type;
use crate::util::get_list_type;
use crate::util::get_string_type;
use crate::util::process_type_signature;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_ir::walker::Visitor;

pub struct TypeStoreInitializer<'a> {
    program: &'a Program,
    group: &'a DependencyGroup<FunctionId>,
    type_store: &'a mut TypeStore,
    type_info_provider: &'a mut TypeInfoProvider,
    errors: &'a mut Vec<TypecheckError>,
}

impl<'a> TypeStoreInitializer<'a> {
    pub fn new(
        program: &'a Program,
        group: &'a DependencyGroup<FunctionId>,
        type_store: &'a mut TypeStore,
        type_info_provider: &'a mut TypeInfoProvider,
        errors: &'a mut Vec<TypecheckError>,
    ) -> TypeStoreInitializer<'a> {
        TypeStoreInitializer {
            program: program,
            group: group,
            type_store: type_store,
            type_info_provider: type_info_provider,
            errors: errors,
        }
    }
}

impl<'a> Visitor for TypeStoreInitializer<'a> {
    fn get_program(&self) -> &Program {
        &self.program
    }

    fn visit_expr(&mut self, expr_id: ExprId, expr: &Expr) {
        //println!("I {} {}", expr_id, expr);
        match expr {
            Expr::ArgRef(_) => {
                let ty = self
                    .type_info_provider
                    .type_var_generator
                    .get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::Bind(_, _) => {
                let ty = self
                    .type_info_provider
                    .type_var_generator
                    .get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::BoolLiteral(_) => {
                self.type_store
                    .initialize_expr(expr_id, get_bool_type(self.program));
            }
            Expr::CaseOf(..) => {
                let ty = self
                    .type_info_provider
                    .type_var_generator
                    .get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::ClassFunctionCall(class_member_id, args) => {
                let class_member_type = self
                    .type_info_provider
                    .get_class_member_type(class_member_id);
                let result_ty = class_member_type.get_result_type(args.len());
                self.type_store
                    .initialize_expr_with_func(expr_id, result_ty, class_member_type);
            }
            Expr::DynamicFunctionCall(_, args) => {
                let mut func_args = Vec::new();
                let (function_type, result_ty) = create_general_function_type(
                    &mut func_args,
                    args.len(),
                    &mut self.type_info_provider.type_var_generator,
                );
                self.type_store
                    .initialize_expr_with_func(expr_id, result_ty, function_type);
            }
            Expr::Do(_) => {
                let ty = self
                    .type_info_provider
                    .type_var_generator
                    .get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::ExprValue(_, _) => {
                let ty = self
                    .type_info_provider
                    .type_var_generator
                    .get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::FieldAccess(..) => {
                let ty = self
                    .type_info_provider
                    .type_var_generator
                    .get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::FloatLiteral(_) => {
                self.type_store
                    .initialize_expr(expr_id, get_float_type(self.program));
            }
            Expr::Formatter(..) => {
                self.type_store
                    .initialize_expr(expr_id, get_string_type(self.program));
            }
            Expr::If(..) => {
                let ty = self
                    .type_info_provider
                    .type_var_generator
                    .get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::IntegerLiteral(_) => {
                self.type_store
                    .initialize_expr(expr_id, get_int_type(self.program));
            }
            Expr::List(_) => {
                let ty = get_list_type(
                    self.program,
                    self.type_info_provider
                        .type_var_generator
                        .get_new_type_var(),
                );
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::StaticFunctionCall(function_id, args) => {
                let function_type = self
                    .type_info_provider
                    .get_function_type(function_id, !self.group.items.contains(function_id));
                let result_ty = function_type.get_result_type(args.len());
                self.type_store
                    .initialize_expr_with_func(expr_id, result_ty, function_type);
            }
            Expr::StringLiteral(_) => {
                self.type_store
                    .initialize_expr(expr_id, get_string_type(self.program));
            }
            Expr::RecordInitialization(id, _) => {
                let record_type_info = self.type_info_provider.get_record_type_info(id);
                let ty = record_type_info.record_type.clone();
                self.type_store
                    .initialize_expr_with_record_type(expr_id, ty, record_type_info);
            }
            Expr::RecordUpdate(..) => {
                let ty = self
                    .type_info_provider
                    .type_var_generator
                    .get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::Tuple(items) => {
                let item_types: Vec<_> = items
                    .iter()
                    .map(|_| {
                        self.type_info_provider
                            .type_var_generator
                            .get_new_type_var()
                    })
                    .collect();
                let tuple_ty = Type::Tuple(item_types);
                self.type_store.initialize_expr(expr_id, tuple_ty);
            }
            Expr::TupleFieldAccess(_, _) => {
                let ty = self
                    .type_info_provider
                    .type_var_generator
                    .get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
        }
    }

    fn visit_pattern(&mut self, pattern_id: PatternId, pattern: &Pattern) {
        //println!("I {} {:?}", pattern_id, pattern);
        match pattern {
            Pattern::Binding(_) => {
                let ty = self
                    .type_info_provider
                    .type_var_generator
                    .get_new_type_var();
                self.type_store.initialize_pattern(pattern_id, ty);
            }
            Pattern::BoolLiteral(_) => {
                self.type_store
                    .initialize_pattern(pattern_id, get_bool_type(self.program));
            }
            Pattern::FloatLiteral(_) => {
                self.type_store
                    .initialize_pattern(pattern_id, get_float_type(self.program));
            }
            Pattern::Guarded(_, _) => {
                let ty = self
                    .type_info_provider
                    .type_var_generator
                    .get_new_type_var();
                self.type_store.initialize_pattern(pattern_id, ty);
            }
            Pattern::IntegerLiteral(_) => {
                self.type_store
                    .initialize_pattern(pattern_id, get_int_type(self.program));
            }
            Pattern::Record(typedef_id, fields) => {
                let record_type_info = self.type_info_provider.get_record_type_info(typedef_id);
                if record_type_info.field_types.len() != fields.len() {
                    let location = self.program.patterns.get(&pattern_id).location_id;
                    let record = self.program.typedefs.get(typedef_id).get_record();
                    let err = TypecheckError::InvalidRecordPattern(
                        location,
                        record.name.clone(),
                        record_type_info.field_types.len(),
                        fields.len(),
                    );
                    self.errors.push(err);
                }
                let ty = record_type_info.record_type.clone();
                self.type_store.initialize_pattern_with_record_type(
                    pattern_id,
                    ty,
                    record_type_info,
                );
            }
            Pattern::StringLiteral(_) => {
                self.type_store
                    .initialize_pattern(pattern_id, get_string_type(self.program));
            }
            Pattern::Typed(_, type_signature) => {
                let ty = process_type_signature(
                    *type_signature,
                    self.program,
                    &mut self.type_info_provider.type_var_generator,
                );
                self.type_store.initialize_pattern(pattern_id, ty);
            }
            Pattern::Tuple(items) => {
                let item_types: Vec<_> = items
                    .iter()
                    .map(|_| {
                        self.type_info_provider
                            .type_var_generator
                            .get_new_type_var()
                    })
                    .collect();
                let tuple_ty = Type::Tuple(item_types);
                self.type_store.initialize_pattern(pattern_id, tuple_ty);
            }
            Pattern::Variant(typedef_id, index, args) => {
                let adt_type_info = self.type_info_provider.get_adt_type_info(typedef_id);
                let variant = &adt_type_info.variant_types[*index];
                if variant.item_types.len() != args.len() {
                    let location = self.program.patterns.get(&pattern_id).location_id;
                    let adt = self.program.typedefs.get(typedef_id).get_adt();
                    let variant_name = adt.variants[*index].name.clone();
                    let err = TypecheckError::InvalidVariantPattern(
                        location,
                        variant_name,
                        variant.item_types.len(),
                        args.len(),
                    );
                    self.errors.push(err);
                }
                let ty = adt_type_info.adt_type.clone();
                self.type_store
                    .initialize_pattern_with_adt_type(pattern_id, ty, adt_type_info);
            }
            Pattern::Wildcard => {
                let ty = self
                    .type_info_provider
                    .type_var_generator
                    .get_new_type_var();
                self.type_store.initialize_pattern(pattern_id, ty);
            }
        }
    }
}
