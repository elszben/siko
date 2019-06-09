use siko_location_info::item::LocationId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum ResolverError {
    ModuleConflict(BTreeMap<String, BTreeSet<LocationId>>),
    InternalModuleConflicts(String, String, Vec<LocationId>),
    ImportedModuleNotFound(String, LocationId),
    UnknownTypeName(String, LocationId),
    UnknownTypeArg(String, LocationId),
    TypeArgumentConflict(Vec<String>, LocationId),
    ArgumentConflict(Vec<String>, LocationId),
    LambdaArgumentConflict(Vec<String>, LocationId),
    UnknownFunction(String, LocationId),
    AmbiguousName(String, LocationId),
    FunctionTypeNameMismatch(String, String, LocationId),
    UnusedTypeArgument(Vec<String>, LocationId),
    RecordFieldNotUnique(String, String, LocationId),
    VariantNotUnique(String, String, LocationId),
    ExportNoMatch(String, String, LocationId),
    ImportNoMatch(String, String, LocationId),
    IncorrectTypeArgumentCount(String, usize, usize, LocationId),
    NameNotType(String, LocationId),
    UnusedHiddenItem(String, String, LocationId),
    UnknownFieldName(String, LocationId),
    NotIrrefutablePattern(LocationId),
    NotRecordType(String, LocationId),
    NoSuchField(String, String, LocationId),
    MissingFields(Vec<String>, LocationId),
    FieldsInitializedMultipleTimes(Vec<String>, LocationId),
    NoRecordFoundWithFields(Vec<String>, LocationId),
    NotAClassName(String, LocationId),
    InvalidArgumentInTypeClassConstraint(String, LocationId),
    NotAClassMember(String, String, LocationId),
    ClassMemberImplementedMultipleTimes(String, LocationId),
    MissingClassMemberInInstance(String, String, LocationId),
}

#[derive(Debug)]
pub struct Error {
    pub errors: Vec<ResolverError>,
}

impl Error {
    pub fn resolve_err(errors: Vec<ResolverError>) -> Error {
        Error { errors: errors }
    }
}
