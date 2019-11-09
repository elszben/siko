use siko_location_info::item::LocationId;

#[derive(Debug)]
pub enum TypecheckError {
    ConflictingInstances(String, LocationId, LocationId),
    DeriveFailureNoInstanceFound(String, String, LocationId),
    DeriveFailureInstanceNotGeneric(String, String, LocationId),
    UntypedExternFunction(String, LocationId),
    FunctionArgAndSignatureMismatch(String, usize, usize, LocationId, bool),
    MainNotFound,
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
