use crate::location_info::item::LocationId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum InternalModuleConflict {
    TypeConflict(String, BTreeSet<LocationId>),
    ItemConflict(String, BTreeSet<LocationId>)
}

#[derive(Debug)]
pub enum ResolverError {
    ModuleConflict(BTreeMap<String, BTreeSet<LocationId>>),
    InternalModuleConflicts(BTreeMap<String, Vec<InternalModuleConflict>>),
    ImportedModuleNotFound(Vec<(String, LocationId)>),
    SymbolNotFoundInModule(String, LocationId),
    UnknownTypeName(String, LocationId),
    TypeArgumentConflict(Vec<String>, LocationId),
    ArgumentConflict(Vec<String>, LocationId),
    LambdaArgumentConflict(Vec<String>, LocationId),
    UnknownFunction(String, LocationId),
    AmbiguousName(String, LocationId),
    FunctionTypeNameMismatch(String, String, LocationId),
    UnusedTypeArgument(Vec<String>, LocationId),
}
