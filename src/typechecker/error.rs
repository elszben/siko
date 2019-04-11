use crate::location_info::item::LocationId;

#[derive(Debug)]
pub enum TypecheckError {
    UntypedExternFunction(String, LocationId),
    FunctionArgAndSignatureMismatch(String, usize, usize, LocationId),
    TooManyArguments(LocationId, String, usize, usize),
    TypeMismatch(LocationId, LocationId, String, String),
    FunctionArgumentMismatch(LocationId, String, String),
    NotCallableType(LocationId, String),
}
