use siko_location_info::item::LocationId;

#[derive(Debug)]
pub enum TypecheckError {
    ConflictingInstances(String, LocationId, LocationId),
    DeriveFailure(String, String, LocationId),
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
