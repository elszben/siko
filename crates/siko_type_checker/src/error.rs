use siko_location_info::item::LocationId;

#[derive(Debug)]
pub enum TypecheckError {
    UntypedExternFunction(String, LocationId),
    FunctionArgAndSignatureMismatch(String, usize, usize, LocationId),
    TypeMismatch(LocationId, String, String),
    FunctionArgumentMismatch(LocationId, String, String),
    RecursiveType(LocationId),
    MainNotFound,
    InvalidFormatString(LocationId),
    AmbiguousFieldAccess(LocationId, Vec<String>),
    InvalidVariantPattern(LocationId, String, usize, usize),
    InvalidRecordPattern(LocationId, String, usize, usize),
}

#[derive(Debug)]
pub struct Error {
    pub errors: Vec<TypecheckError>,
}

impl Error {
    pub fn typecheck_err(errors: Vec<TypecheckError>) -> Error {
        Error { errors: errors }
    }
}
