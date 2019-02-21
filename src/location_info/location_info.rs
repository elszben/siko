use crate::location_info::location_set::LocationSet;
use crate::syntax::expr::ExprId;
use crate::syntax::function::FunctionId;
use crate::syntax::import::ImportId;
use crate::syntax::module::ModuleId;
use crate::syntax::types::TypeSignatureId;
use std::collections::BTreeMap;

pub struct Module {
    pub location: LocationSet,
}

impl Module {
    pub fn new(location: LocationSet) -> Module {
        Module { location: location }
    }
}

pub struct Function {
    pub location: LocationSet,
}

impl Function {
    pub fn new(location: LocationSet) -> Function {
        Function { location: location }
    }
}

pub struct Import {
    pub location: LocationSet,
}

impl Import {
    pub fn new(location: LocationSet) -> Import {
        Import { location: location }
    }
}

pub struct Expr {
    pub location: LocationSet,
}

impl Expr {
    pub fn new(location: LocationSet) -> Expr {
        Expr { location: location }
    }
}

pub struct TypeSignature {
    pub location: LocationSet,
}

impl TypeSignature {
    pub fn new(location: LocationSet) -> TypeSignature {
        TypeSignature { location: location }
    }
}

pub struct LocationInfo {
    modules: BTreeMap<ModuleId, Module>,
    functions: BTreeMap<FunctionId, Function>,
    imports: BTreeMap<ImportId, Import>,
    exprs: BTreeMap<ExprId, Expr>,
    type_signatures: BTreeMap<TypeSignatureId, TypeSignature>,
}

impl LocationInfo {
    pub fn new() -> LocationInfo {
        LocationInfo {
            modules: BTreeMap::new(),
            functions: BTreeMap::new(),
            imports: BTreeMap::new(),
            exprs: BTreeMap::new(),
            type_signatures: BTreeMap::new(),
        }
    }

    pub fn add_module(&mut self, id: ModuleId, module: Module) {
        self.modules.insert(id, module);
    }

    pub fn add_function(&mut self, id: FunctionId, function: Function) {
        self.functions.insert(id, function);
    }

    pub fn add_import(&mut self, id: ImportId, import: Import) {
        self.imports.insert(id, import);
    }

    pub fn add_expr(&mut self, id: ExprId, expr: Expr) {
        self.exprs.insert(id, expr);
    }

    pub fn add_type_signature(&mut self, id: TypeSignatureId, ts: TypeSignature) {
        self.type_signatures.insert(id, ts);
    }

    pub fn get_module_location(&self, id: &ModuleId) -> &LocationSet {
        &self.modules.get(id).expect("Module not found").location
    }

    pub fn get_import_location(&self, id: &ImportId) -> &LocationSet {
        &self.imports.get(id).expect("Import not found").location
    }

    pub fn get_function_location(&self, id: &FunctionId) -> &LocationSet {
        &self.functions.get(id).expect("Function not found").location
    }

    pub fn get_expr_location(&self, id: &ExprId) -> &LocationSet {
        &self.exprs.get(id).expect("Exor not found").location
    }

    pub fn get_type_signature_location(&self, id: &TypeSignatureId) -> &LocationSet {
        &self
            .type_signatures
            .get(id)
            .expect("TypeSignature not found")
            .location
    }
}
