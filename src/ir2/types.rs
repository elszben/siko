use crate::syntax::types::TypeSignatureId as AstTypeSignatureId;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeSignatureId {
    pub id: usize,
}

#[derive(Debug, Clone)]
pub enum TypeSignature {
    Bool,
    Int,
    String,
    Nothing,
    Tuple(Vec<TypeSignatureId>),
    Function(Vec<TypeSignatureId>),
    TypeArgument(String),
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub type_signature: TypeSignature,
    pub ast_type_id: AstTypeSignatureId,
}

impl TypeInfo {
    pub fn new(type_signature: TypeSignature, ast_type_id: AstTypeSignatureId) -> TypeInfo {
        TypeInfo {
            type_signature: type_signature,
            ast_type_id: ast_type_id,
        }
    }
}
