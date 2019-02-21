use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::expr::ExprInfo;
use crate::ir::function::Function;
use crate::ir::function::FunctionId;
use crate::ir::types::TypeInfo;
use crate::ir::types::TypeSignature;
use crate::ir::types::TypeSignatureId;

use crate::util::Counter;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Program {
    pub type_signatures: BTreeMap<TypeSignatureId, TypeInfo>,
    pub exprs: BTreeMap<ExprId, ExprInfo>,
    pub functions: BTreeMap<FunctionId, Function>,
    type_signature_id: Counter,
    expr_id: Counter,
    function_id: Counter,
}

impl Program {
    pub fn new() -> Program {
        Program {
            type_signatures: BTreeMap::new(),
            exprs: BTreeMap::new(),
            functions: BTreeMap::new(),
            type_signature_id: Counter::new(),
            expr_id: Counter::new(),
            function_id: Counter::new(),
        }
    }

    pub fn get_type_signature_id(&mut self) -> TypeSignatureId {
        TypeSignatureId {
            id: self.type_signature_id.next(),
        }
    }

    pub fn get_expr_id(&mut self) -> ExprId {
        ExprId {
            id: self.expr_id.next(),
        }
    }

    pub fn get_function_id(&mut self) -> FunctionId {
        FunctionId {
            id: self.function_id.next(),
        }
    }

    pub fn add_type_signature(&mut self, id: TypeSignatureId, type_info: TypeInfo) {
        self.type_signatures.insert(id, type_info);
    }

    pub fn get_type_signature(&self, id: &TypeSignatureId) -> &TypeSignature {
        &self
            .type_signatures
            .get(id)
            .expect("TypeSignature not found")
            .type_signature
    }

    pub fn add_expr(&mut self, id: ExprId, expr_info: ExprInfo) {
        self.exprs.insert(id, expr_info);
    }

    pub fn get_expr(&self, id: &ExprId) -> &Expr {
        &self.exprs.get(id).expect("Expr not found").expr
    }

    pub fn add_function(&mut self, id: FunctionId, function: Function) {
        self.functions.insert(id, function);
    }

    pub fn get_function(&self, id: &FunctionId) -> &Function {
        &self.functions.get(id).expect("Function not found")
    }
}
