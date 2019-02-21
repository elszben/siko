#[derive(Debug, Clone)]
pub enum TypeCheckerError {
    TypeArgumentMismatch,
    TooManyArguments,
}
