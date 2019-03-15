use crate::location_info::item::LocationId;

#[derive(Debug)]
pub enum TypecheckError {
    UntypedExternFunction(String, LocationId),
    FunctionTypeDependencyLoop,
    TooManyArguments(LocationId, String, usize, usize),
    TypeMismatch(LocationId, String, String),
    FunctionArgumentMismatch(LocationId, String, String),
    NotCallableType(LocationId, String),
}
