use crate::common::AdtTypeInfo;
use crate::common::ClassMemberTypeInfo;
use crate::common::FunctionTypeInfoStore;
use crate::common::RecordTypeInfo;
use crate::dependency_processor::DependencyGroup;
use crate::error::TypecheckError;
use crate::type_store::TypeStore;
use crate::type_var_generator::TypeVarGenerator;
use crate::types::Type;
use crate::util::create_general_function_type;
use crate::util::get_bool_type;
use crate::util::get_float_type;
use crate::util::get_int_type;
use crate::util::get_list_type;
use crate::util::get_string_type;
use siko_ir::class::ClassMemberId;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_ir::types::TypeDefId;
use siko_ir::walker::Visitor;
use std::collections::BTreeMap;

pub struct TypeStoreInitializer<'a> {
    program: &'a Program,
    group: &'a DependencyGroup<FunctionId>,
    type_store: &'a mut TypeStore,
    function_type_info_store: &'a mut FunctionTypeInfoStore,
    class_member_type_info_map: &'a BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
    adt_type_info_map: &'a BTreeMap<TypeDefId, AdtTypeInfo>,
    record_type_info_map: &'a BTreeMap<TypeDefId, RecordTypeInfo>,
    type_var_generator: TypeVarGenerator,
    errors: &'a mut Vec<TypecheckError>,
}

impl<'a> TypeStoreInitializer<'a> {
    pub fn new(
        program: &'a Program,
        group: &'a DependencyGroup<FunctionId>,
        type_store: &'a mut TypeStore,
        function_type_info_store: &'a mut FunctionTypeInfoStore,
        class_member_type_info_map: &'a BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
        adt_type_info_map: &'a BTreeMap<TypeDefId, AdtTypeInfo>,
        record_type_info_map: &'a BTreeMap<TypeDefId, RecordTypeInfo>,
        type_var_generator: TypeVarGenerator,
        errors: &'a mut Vec<TypecheckError>,
    ) -> TypeStoreInitializer<'a> {
        TypeStoreInitializer {
            program: program,
            group: group,
            type_store: type_store,
            function_type_info_store: function_type_info_store,
            class_member_type_info_map: class_member_type_info_map,
            type_var_generator: type_var_generator,
            adt_type_info_map: adt_type_info_map,
            record_type_info_map: record_type_info_map,
            errors: errors,
        }
    }

    fn clone_type(&mut self, ty: &Type) -> Type {
        let mut arg_map = BTreeMap::new();
        ty.duplicate(&mut arg_map, &mut self.type_var_generator)
            .remove_fixed_types()
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
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::Bind(_, _) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::BoolLiteral(_) => {
                self.type_store
                    .initialize_expr(expr_id, get_bool_type(self.program));
            }
            Expr::CaseOf(..) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::ClassFunctionCall(class_member_id, args) => {
                let class_member_type_info = self
                    .class_member_type_info_map
                    .get(class_member_id)
                    .expect("Class member type info not found");
                let mut arg_map = BTreeMap::new();
                let function_type = class_member_type_info
                    .ty
                    .duplicate(&mut arg_map, &mut self.type_var_generator)
                    .remove_fixed_types();
                let result_ty = function_type.get_result_type(args.len());
                self.type_store
                    .initialize_expr_with_func(expr_id, result_ty, function_type);
            }
            Expr::DynamicFunctionCall(_, args) => {
                let mut func_args = Vec::new();
                let (function_type, result_ty) = create_general_function_type(
                    &mut func_args,
                    args.len(),
                    &mut self.type_var_generator,
                );
                self.type_store
                    .initialize_expr_with_func(expr_id, result_ty, function_type);
            }
            Expr::Do(_) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::ExprValue(_, _) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::FieldAccess(..) => {
                let ty = self.type_var_generator.get_new_type_var();
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
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::IntegerLiteral(_) => {
                self.type_store
                    .initialize_expr(expr_id, get_int_type(self.program));
            }
            Expr::List(_) => {
                let ty = get_list_type(self.program, self.type_var_generator.get_new_type_var());
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::StaticFunctionCall(function_id, args) => {
                let function_type = if self.group.items.contains(function_id) {
                    self.function_type_info_store
                        .get(function_id)
                        .function_type
                        .clone()
                } else {
                    let ty = self
                        .function_type_info_store
                        .get(function_id)
                        .function_type
                        .clone();
                    self.clone_type(&ty)
                };
                let result_ty = function_type.get_result_type(args.len());
                self.type_store
                    .initialize_expr_with_func(expr_id, result_ty, function_type);
            }
            Expr::StringLiteral(_) => {
                self.type_store
                    .initialize_expr(expr_id, get_string_type(self.program));
            }
            Expr::RecordInitialization(id, _) => {
                let record_type_info = self
                    .record_type_info_map
                    .get(id)
                    .expect("Record type info not found");
                let record_type_info = record_type_info.duplicate(&mut self.type_var_generator);
                let ty = record_type_info.record_type.clone();
                self.type_store
                    .initialize_expr_with_record_type(expr_id, ty, record_type_info);
            }
            Expr::Tuple(items) => {
                let item_types: Vec<_> = items
                    .iter()
                    .map(|_| self.type_var_generator.get_new_type_var())
                    .collect();
                let tuple_ty = Type::Tuple(item_types);
                self.type_store.initialize_expr(expr_id, tuple_ty);
            }
            Expr::TupleFieldAccess(_, _) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            _ => panic!("init of {} not yet implemented", expr),
        }
    }

    fn visit_pattern(&mut self, pattern_id: PatternId, pattern: &Pattern) {
        //println!("I {} {:?}", pattern_id, pattern);
        match pattern {
            Pattern::Binding(_) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_pattern(pattern_id, ty);
            }
            Pattern::BoolLiteral(_) => {
                self.type_store
                    .initialize_pattern(pattern_id, get_bool_type(self.program));
            }
            Pattern::Tuple(items) => {
                let item_types: Vec<_> = items
                    .iter()
                    .map(|_| self.type_var_generator.get_new_type_var())
                    .collect();
                let tuple_ty = Type::Tuple(item_types);
                self.type_store.initialize_pattern(pattern_id, tuple_ty);
            }
            Pattern::Variant(typedef_id, index, args) => {
                let adt = self.program.typedefs.get(typedef_id).get_adt();
                let variant = &adt.variants[*index];
                if variant.items.len() != args.len() {
                    let location = self.program.patterns.get(&pattern_id).location_id;
                    let err = TypecheckError::InvalidVariantPattern(
                        location,
                        variant.name.clone(),
                        variant.items.len(),
                        args.len(),
                    );
                    self.errors.push(err);
                }
                let adt_type_info = self
                    .adt_type_info_map
                    .get(typedef_id)
                    .expect("Adt type info not found");
                let adt_type_info = adt_type_info.duplicate(&mut self.type_var_generator);
                let ty = adt_type_info.adt_type.clone();
                self.type_store
                    .initialize_pattern_with_adt_type(pattern_id, ty, adt_type_info);
            }
            Pattern::Wildcard => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_pattern(pattern_id, ty);
            }
            _ => panic!("{:?} NYI", pattern),
        }
    }
}
