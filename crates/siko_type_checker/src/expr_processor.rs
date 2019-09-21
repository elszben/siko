use crate::common::ClassMemberTypeInfo;
use crate::common::DependencyGroup;
use crate::common::FunctionTypeInfo;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::error::TypecheckError;
use crate::type_store::TypeStore;
use crate::type_variable::TypeVariable;
use crate::types::Type;
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
use siko_ir::types::Type as IrType;
use siko_ir::types::TypeDefId;
use siko_ir::types::TypeId as IrTypeId;
use siko_location_info::item::LocationId;
use std::collections::BTreeMap;

pub fn convert_to_ir_type(
    var: &TypeVariable,
    program: &mut Program,
    type_store: &TypeStore,
) -> IrTypeId {
    let expr_ty = type_store.get_type(var);
    let ir_type = match expr_ty {
        Type::TypeArgument(arg, constraints) => IrType::TypeArgument(arg, constraints),
        Type::FixedTypeArgument(arg, _, constraints) => IrType::TypeArgument(arg, constraints),
        Type::Function(func_type) => {
            let from = convert_to_ir_type(&func_type.from, program, type_store);
            let to = convert_to_ir_type(&func_type.to, program, type_store);
            IrType::Function(from, to)
        }
        Type::Named(name, def_id, items) => {
            let items: Vec<_> = items
                .iter()
                .map(|v| convert_to_ir_type(v, program, type_store))
                .collect();
            IrType::Named(name, def_id, items)
        }
        Type::Tuple(items) => {
            let items: Vec<_> = items
                .iter()
                .map(|v| convert_to_ir_type(v, program, type_store))
                .collect();
            IrType::Tuple(items)
        }
    };
    let ir_type_id = IrTypeId::from(var.id);
    program.types.insert(ir_type_id, ir_type);

    ir_type_id
}

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

    pub fn export_expr_types(&mut self) {
        for (expr_id, var) in &self.expression_type_var_map {
            let ir_type_id = convert_to_ir_type(var, &mut self.program, &self.type_store);
            self.program.expr_types.insert(*expr_id, ir_type_id);
        }
    }
}
