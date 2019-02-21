use crate::syntax::expr::Expr;
use crate::syntax::expr::ExprId;
use crate::syntax::function::FunctionId;
use crate::syntax::import::ImportId;
use crate::syntax::module::Module;
use crate::syntax::module::ModuleId;
use crate::syntax::types::TypeSignature;
use crate::syntax::types::TypeSignatureId;
use crate::util::Counter;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct Program {
    pub modules: BTreeMap<ModuleId, Module>,
    pub exprs: BTreeMap<ExprId, Expr>,
    pub type_signatures: BTreeMap<TypeSignatureId, TypeSignature>,
    module_id: Counter,
    function_id: Counter,
    import_id: Counter,
    expr_id: Counter,
    type_signature_id: Counter,
}

impl Program {
    pub fn new() -> Program {
        Program {
            modules: BTreeMap::new(),
            exprs: BTreeMap::new(),
            type_signatures: BTreeMap::new(),
            module_id: Counter::new(),
            function_id: Counter::new(),
            import_id: Counter::new(),
            expr_id: Counter::new(),
            type_signature_id: Counter::new(),
        }
    }

    pub fn get_module_id(&mut self) -> ModuleId {
        ModuleId {
            id: self.module_id.next(),
        }
    }

    pub fn get_function_id(&mut self) -> FunctionId {
        FunctionId {
            id: self.module_id.next(),
        }
    }

    pub fn get_import_id(&mut self) -> ImportId {
        ImportId {
            id: self.import_id.next(),
        }
    }

    pub fn get_expr_id(&mut self) -> ExprId {
        ExprId {
            id: self.expr_id.next(),
        }
    }

    pub fn get_type_signature_id(&mut self) -> TypeSignatureId {
        TypeSignatureId {
            id: self.type_signature_id.next(),
        }
    }

    pub fn add_module(&mut self, id: ModuleId, module: Module) {
        self.modules.insert(id, module);
    }

    pub fn add_expr(&mut self, id: ExprId, expr: Expr) {
        self.exprs.insert(id, expr);
    }

    pub fn add_type_signature(&mut self, id: TypeSignatureId, type_signature: TypeSignature) {
        self.type_signatures.insert(id, type_signature);
    }

    pub fn get_module(&self, id: &ModuleId) -> &Module {
        &self.modules.get(id).expect("Module not found")
    }

    pub fn get_expr(&self, id: &ExprId) -> &Expr {
        &self.exprs.get(id).expect("Expr not found")
    }

    pub fn get_type_signature(&self, id: &TypeSignatureId) -> &TypeSignature {
        &self
            .type_signatures
            .get(id)
            .expect("TypeSignature not found")
    }
}
