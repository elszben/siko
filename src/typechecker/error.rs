use crate::location_info::item::LocationId;

#[derive(Debug)]
pub enum TypecheckError {
    UntypedExternFunction(String, LocationId),
    FunctionArgAndSignatureMismatch(String, usize, usize, LocationId),
    TypeMismatch(LocationId, LocationId, String, String),
    FunctionArgumentMismatch(LocationId, String, String),
    RecursiveType(LocationId),
    MainNotFound,
    InvalidFormatString(LocationId),
    AmbiguousFieldAccess(LocationId, Vec<String>),
}
