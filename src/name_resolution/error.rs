use crate::location_info::item::LocationId;
use crate::syntax::expr::ExprId;
use crate::syntax::function::FunctionId;
use crate::syntax::import::ImportId;
use crate::syntax::types::TypeSignatureId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum ResolverError {
    ModuleConflict(BTreeMap<String, BTreeSet<LocationId>>),
    ImportedModuleNotFound(Vec<(String, ImportId)>),
    SymbolNotFoundInModule(String, ImportId),
    UnknownTypeName(String, TypeSignatureId),
    TypeArgumentConflict(Vec<String>, TypeSignatureId),
    ArgumentConflict(Vec<String>, FunctionId),
    LambdaArgumentConflict(Vec<String>, ExprId),
    UnknownFunction(String, ExprId),
    AmbiguousName(String, ExprId),
    FunctionTypeNameMismatch(String, String, TypeSignatureId),
    UnusedTypeArgument(Vec<String>, TypeSignatureId),
}
