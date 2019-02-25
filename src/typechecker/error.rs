use crate::syntax::function::FunctionId;

#[derive(Debug)]
pub enum TypecheckError {
    UntypedExternFunction(String, FunctionId),
    FunctionTypeDependencyLoop,
}
