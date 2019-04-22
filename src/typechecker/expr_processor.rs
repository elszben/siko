use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::expr::FunctionArgumentRef;
use crate::ir::function::FunctionId;
use crate::ir::program::Program;
use crate::location_info::item::LocationId;
use crate::typechecker::common::DependencyGroup;
use crate::typechecker::common::FunctionTypeInfo;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use crate::typechecker::walker::walk_expr;
use crate::typechecker::walker::Visitor;
use crate::util::format_list;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

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
}

impl<'a> Unifier<'a> {
    fn new(expr_processor: &'a mut ExprProcessor) -> Unifier<'a> {
        Unifier {
            expr_processor: expr_processor,
        }
    }
}

impl<'a> Visitor for Unifier<'a> {
    fn visit(&mut self, expr_id: ExprId, expr: &Expr) {
        unreachable!();
        match expr {
            _ => unimplemented!(),
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

    pub fn process_untyped_dep_group(&mut self, program: &Program, group: &DependencyGroup) {
        for function in &group.functions {
            self.process_untyped_function(function, program);
        }
    }

    pub fn process_untyped_function(&mut self, function_id: &FunctionId, program: &Program) {
        let type_info = self
            .function_type_info_map
            .get(function_id)
            .expect("Function type info not found");
        let body = type_info.body.expect("body not found");
        let mut type_var_creator = TypeVarCreator::new(self);
        walk_expr(&body, program, &mut type_var_creator);
        let mut unifier = Unifier::new(self);
        walk_expr(&body, program, &mut unifier);
    }

    pub fn dump_everything(&self, program: &Program) {
        for (id, info) in &self.function_type_info_map {
            println!(
                "{}/{}: {}",
                id,
                info.displayed_name,
                self.type_store
                    .get_resolved_type_string(&info.function_type)
            );
        }

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
}
