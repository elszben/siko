use crate::location_info::item::LocationId;
use crate::syntax::types::TypeSignatureId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum ResolverError {
    ModuleConflict(BTreeMap<String, BTreeSet<LocationId>>),
    ImportedModuleNotFound(Vec<(String, LocationId)>),
    SymbolNotFoundInModule(String, LocationId),
    UnknownTypeName(String, TypeSignatureId),
    TypeArgumentConflict(Vec<String>, TypeSignatureId),
    ArgumentConflict(Vec<String>, LocationId),
    LambdaArgumentConflict(Vec<String>, LocationId),
    UnknownFunction(String, LocationId),
    AmbiguousName(String, LocationId),
    FunctionTypeNameMismatch(String, String, TypeSignatureId),
    UnusedTypeArgument(Vec<String>, TypeSignatureId),
}
