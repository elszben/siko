use crate::common::ClassMemberTypeInfo;
use crate::common::DependencyGroup;
use crate::common::FunctionTypeInfo;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::error::TypecheckError;
use crate::type_store::TypeStore;
use crate::type_variable::TypeVariable;
use crate::unifier::Unifier;
use crate::walker::walk_expr;
use crate::walker::Visitor;
use siko_ir::class::ClassMemberId;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_ir::types::TypeDefId;
use siko_location_info::item::LocationId;
use std::collections::BTreeMap;

struct TypeVarCreator<'a, 'b> {
    expr_processor: &'a mut ExprProcessor<'b>,
}

impl<'a, 'b: 'a> TypeVarCreator<'a, 'b> {
    fn new(expr_processor: &'a mut ExprProcessor<'b>) -> TypeVarCreator<'a, 'b> {
        TypeVarCreator {
            expr_processor: expr_processor,
        }
    }
}

impl<'a, 'b> Visitor for TypeVarCreator<'a, 'b> {
    fn get_program(&self) -> &Program {
        &self.expr_processor.program
    }

    fn visit_expr(&mut self, expr_id: ExprId, _: &Expr) {
        self.expr_processor.create_type_var_for_expr(expr_id);
    }

    fn visit_pattern(&mut self, pattern_id: PatternId, _: &Pattern) {
        self.expr_processor.create_type_var_for_pattern(pattern_id);
    }
}

pub struct ExprProcessor<'a> {
    pub type_store: TypeStore,
    expression_type_var_map: BTreeMap<ExprId, TypeVariable>,
    pattern_type_var_map: BTreeMap<PatternId, TypeVariable>,
    pub function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    pub record_type_info_map: BTreeMap<TypeDefId, RecordTypeInfo>,
    pub variant_type_info_map: BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    pub class_member_type_info_map: BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
    pub program: &'a mut Program,
}

impl<'a> ExprProcessor<'a> {
    pub fn new(
        type_store: TypeStore,
        function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
        record_type_info_map: BTreeMap<TypeDefId, RecordTypeInfo>,
        variant_type_info_map: BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
        class_member_type_info_map: BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
        program: &'a mut Program,
    ) -> ExprProcessor<'a> {
        ExprProcessor {
            type_store: type_store,
            expression_type_var_map: BTreeMap::new(),
            pattern_type_var_map: BTreeMap::new(),
            function_type_info_map: function_type_info_map,
            record_type_info_map: record_type_info_map,
            variant_type_info_map: variant_type_info_map,
            class_member_type_info_map: class_member_type_info_map,
            program: program,
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

    pub fn process_dep_group(&mut self, group: &DependencyGroup, errors: &mut Vec<TypecheckError>) {
        for function in &group.functions {
            self.process_function(function, errors, group);
        }
    }

    pub fn process_function(
        &mut self,
        function_id: &FunctionId,
        errors: &mut Vec<TypecheckError>,
        group: &DependencyGroup,
    ) {
        let type_info = self
            .function_type_info_map
            .get(function_id)
            .expect("Function type info not found");
        let body = type_info.body.expect("body not found");
        let arg_map = type_info.arg_map.clone();
        let result_var = type_info.result;
        let mut type_var_creator = TypeVarCreator::new(self);
        walk_expr(&body, &mut type_var_creator);
        let mut unifier = Unifier::new(self, errors, group, arg_map);
        walk_expr(&body, &mut unifier);
        let body_var = self.lookup_type_var_for_expr(&body);
        let body_location = self.program.exprs.get(&body).location_id;
        self.unify_variables(&result_var, &body_var, body_location, errors);
    }

    #[allow(unused)]
    pub fn dump_expression_types(&self, program: &Program) {
        for (expr_id, expr_info) in &program.exprs.items {
            let var = self.lookup_type_var_for_expr(expr_id);
            println!(
                "Expr: {}: {} -> {}",
                expr_id,
                expr_info.item,
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

    pub fn unify_variables(
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
