use crate::ir::expr::ExprId;
use crate::ir::types::TypeDefId;
use crate::ir::types::TypeSignatureId;
use crate::location_info::item::LocationId;
use std::fmt;
use crate::ir::class::ClassId;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct FunctionId {
    pub id: usize,
}

impl fmt::Display for FunctionId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "func#{}", self.id)
    }
}

#[derive(Debug, Clone)]
pub struct NamedFunctionInfo {
    pub body: Option<ExprId>,
    pub module: String,
    pub name: String,
    pub type_signature: Option<TypeSignatureId>,
    pub location_id: LocationId,
}

impl fmt::Display for NamedFunctionInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.module, self.name)
    }
}

#[derive(Debug, Clone)]
pub struct LambdaInfo {
    pub body: ExprId,
    pub host_info: String,
    pub host_function: FunctionId,
    pub index: usize,
    pub location_id: LocationId,
}

impl fmt::Display for LambdaInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/lambda#{}", self.host_info, self.index)
    }
}

#[derive(Debug, Clone)]
pub struct RecordConstructorInfo {
    pub type_id: TypeDefId,
}

impl fmt::Display for RecordConstructorInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.type_id)
    }
}

#[derive(Debug, Clone)]
pub struct VariantConstructorInfo {
    pub type_id: TypeDefId,
    pub index: usize,
}

impl fmt::Display for VariantConstructorInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.type_id, self.index)
    }
}


#[derive(Debug, Clone)]
pub struct ClassMemberInfo {
    pub class_id: ClassId,
    pub index: usize,
}

impl fmt::Display for ClassMemberInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.class_id, self.index)
    }
}


#[derive(Debug, Clone)]
pub enum FunctionInfo {
    Lambda(LambdaInfo),
    NamedFunction(NamedFunctionInfo),
    RecordConstructor(RecordConstructorInfo),
    VariantConstructor(VariantConstructorInfo),
    ClassMember(ClassMemberInfo)
}

impl fmt::Display for FunctionInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FunctionInfo::Lambda(i) => write!(f, "lambda{}", i),
            FunctionInfo::NamedFunction(i) => write!(f, "{}", i),
            FunctionInfo::RecordConstructor(i) => write!(f, "{}", i),
            FunctionInfo::VariantConstructor(i) => write!(f, "{}", i),
            FunctionInfo::ClassMember(i) => write!(f, "{}", i),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub id: FunctionId,
    pub arg_locations: Vec<LocationId>,
    pub implicit_arg_count: usize,
    pub info: FunctionInfo,
}

impl Function {
    pub fn get_lambda_host(&self) -> Option<FunctionId> {
        match &self.info {
            FunctionInfo::Lambda(i) => Some(i.host_function),
            _ => None,
        }
    }
}
