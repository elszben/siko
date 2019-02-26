use crate::ir::program::Program;
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
    Function(FunctionType),
    TypeArgument(usize),
}
impl TypeSignature {
    pub fn format(&self, program: &Program) -> String {
        format!("")
    }
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

#[derive(Debug, Clone)]
pub struct FunctionType {
    pub types: Vec<TypeSignatureId>,
}

impl FunctionType {
    pub fn new(types: Vec<TypeSignatureId>) -> FunctionType {
        FunctionType { types: types }
    }

    pub fn get_return_type(&self) -> TypeSignatureId {
        self.types.last().expect("empty function!").clone()
    }

    pub fn get_arg_count(&self) -> usize {
        self.types.len() - 1
    }

    pub fn format(&self, program: &Program) -> String {
        if self.get_arg_count() != 0 {
            let ss: Vec<_> = self
                .types
                .iter()
                .map(|t| {
                    let type_signature = program.get_type_signature(t);
                    format!("{}", type_signature.format(program))
                })
                .collect();
            format!("{}", ss.join(" -> "))
        } else {
            let t = self.get_return_type();
            let type_signature = program.get_type_signature(&t);
            format!("{}", type_signature.format(program))
        }
    }
}
