use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::expr::FunctionArgumentRef;
use crate::ir::function::FunctionId;
use crate::ir::program::Program;
use crate::location_info::item::LocationId;
use crate::typechecker::common::FunctionTypeInfo;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use crate::util::format_list;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

enum Constraint {
    Int(TypeVariable),
    String(TypeVariable),
    Bool(TypeVariable),
    FunctionResult(TypeVariable, TypeVariable),
}

pub struct ExprProcessor {
    type_store: TypeStore,
    expression_type_var_map: BTreeMap<ExprId, TypeVariable>,
    function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    constraints: Vec<Constraint>,
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
            constraints: Vec::new(),
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

    pub fn process_expr_and_create_vars(&mut self, program: &Program) {
        for (expr_id, expr_info) in &program.exprs {
            println!("Processing {} {}", expr_id, expr_info.expr);
            self.create_type_var_for_expr(*expr_id);
        }

        for (expr_id, expr_info) in &program.exprs {
            println!("Creating constraint for {} {}", expr_id, expr_info.expr);
            let c = match expr_info.expr {
                Expr::IntegerLiteral(_) => Constraint::Int(self.lookup_type_var_for_expr(expr_id)),
                Expr::StringLiteral(_) => {
                    Constraint::String(self.lookup_type_var_for_expr(expr_id))
                }
                Expr::BoolLiteral(_) => Constraint::Bool(self.lookup_type_var_for_expr(expr_id)),
                _ => unreachable!(),
            };
            self.constraints.push(c);
        }

        for (id, info) in &self.function_type_info_map {
            let body = if let Some(body) = info.body {
                body
            } else {
                continue;
            };
            let body_var = self.lookup_type_var_for_expr(&body);
            let c = Constraint::FunctionResult(body_var, info.result);
            self.constraints.push(c);
        }
    }

    pub fn check_constraints(&mut self, program: &Program, errors: &mut Vec<TypecheckError>) {
        for c in &self.constraints {
            match c {
                Constraint::Int(var) => {
                    if !self.type_store.set_type(var, Type::Int) {
                        unimplemented!()
                    }
                }
                Constraint::FunctionResult(body, result) => {
                    if !self.type_store.unify(body, result) {
                        unimplemented!()
                    }
                }
                _ => unimplemented!(),
            }
        }
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
