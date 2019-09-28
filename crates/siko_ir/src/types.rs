use crate::class::ClassId;
use crate::class::InstanceId;
use crate::function::FunctionId;
use crate::program::Program;
use siko_location_info::item::LocationId;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeSignatureId {
    pub id: usize,
}

impl From<usize> for TypeSignatureId {
    fn from(id: usize) -> TypeSignatureId {
        TypeSignatureId { id: id }
    }
}

#[derive(Debug, Clone)]
pub enum TypeSignature {
    Tuple(Vec<TypeSignatureId>),
    Function(TypeSignatureId, TypeSignatureId),
    TypeArgument(usize, String, Vec<ClassId>),
    Named(String, TypeDefId, Vec<TypeSignatureId>),
    Variant(String, Vec<TypeSignatureId>),
    Wildcard,
}

#[derive(Debug, Clone)]
pub struct RecordField {
    pub name: String,
    pub type_signature_id: TypeSignatureId,
}

#[derive(Debug, Clone)]
pub struct Record {
    pub name: String,
    pub id: TypeDefId,
    pub type_args: Vec<usize>,
    pub fields: Vec<RecordField>,
    pub constructor: FunctionId,
    pub location_id: LocationId,
}

#[derive(Debug, Clone)]
pub struct VariantItem {
    pub type_signature_id: TypeSignatureId,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub items: Vec<VariantItem>,
    pub type_signature_id: TypeSignatureId,
    pub constructor: FunctionId,
}

#[derive(Debug, Clone)]
pub struct Adt {
    pub name: String,
    pub id: TypeDefId,
    pub type_args: Vec<usize>,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Clone)]
pub enum TypeDef {
    Record(Record),
    Adt(Adt),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeDefId {
    pub id: usize,
}

impl fmt::Display for TypeDefId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TypeDefId({})", self.id)
    }
}

impl From<usize> for TypeDefId {
    fn from(id: usize) -> TypeDefId {
        TypeDefId { id: id }
    }
}

impl TypeDef {
    pub fn get_adt(&self) -> &Adt {
        if let TypeDef::Adt(adt) = self {
            &adt
        } else {
            unreachable!()
        }
    }

    pub fn get_record(&self) -> &Record {
        if let TypeDef::Record(record) = self {
            &record
        } else {
            unreachable!()
        }
    }

    pub fn get_mut_adt(&mut self) -> &mut Adt {
        if let TypeDef::Adt(ref mut adt) = self {
            adt
        } else {
            unreachable!()
        }
    }

    pub fn get_mut_record(&mut self) -> &mut Record {
        if let TypeDef::Record(ref mut record) = self {
            record
        } else {
            unreachable!()
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct TypeId {
    pub id: usize,
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "TypeId({})", self.id)
    }
}

impl From<usize> for TypeId {
    fn from(id: usize) -> TypeId {
        TypeId { id: id }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum ConcreteType {
    Tuple(Vec<ConcreteType>),
    Named(String, TypeDefId, Vec<ConcreteType>),
    Function(Box<ConcreteType>, Box<ConcreteType>),
}

impl fmt::Display for ConcreteType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConcreteType::Tuple(items) => {
                let ss: Vec<_> = items.iter().map(|i| format!("{}", i)).collect();
                write!(f, "({})", ss.join(", "))
            }
            ConcreteType::Named(name, _, items) => {
                let ss: Vec<_> = items.iter().map(|i| format!("{}", i)).collect();
                let args = if ss.is_empty() {
                    "".to_string()
                } else {
                    format!(" {}", ss.join(" "))
                };
                write!(f, "{}{}", name, args)
            }
            ConcreteType::Function(from, to) => write!(f, "{} -> {}", from, to),
        }
    }
}

#[derive(Debug)]
pub struct TypeInstanceResolver {
    pub instance_map: BTreeMap<ClassId, BTreeMap<ConcreteType, InstanceId>>,
}

impl TypeInstanceResolver {
    pub fn new() -> TypeInstanceResolver {
        TypeInstanceResolver {
            instance_map: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, class_id: ClassId, ty: ConcreteType, instance_id: InstanceId) {
        let types = self
            .instance_map
            .entry(class_id)
            .or_insert_with(|| BTreeMap::new());
        types.insert(ty, instance_id);
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FunctionType {
    pub from: TypeId,
    pub to: TypeId,
}

impl FunctionType {
    pub fn new(from: TypeId, to: TypeId) -> FunctionType {
        FunctionType { from: from, to: to }
    }

    pub fn get_return_type(&self, program: &Program, arg_count: usize) -> TypeId {
        if arg_count == 1 {
            self.to
        } else {
            if let Type::Function(to_func_type) =
                program.types.get(&self.to).expect("Type not found")
            {
                to_func_type.get_return_type(program, arg_count - 1)
            } else {
                self.to
            }
        }
    }

    pub fn get_arg_types(&self, program: &Program, arg_vars: &mut Vec<TypeId>) {
        arg_vars.push(self.from);
        if let Type::Function(to_func_type) = program.types.get(&self.to).expect("Type not found") {
            to_func_type.get_arg_types(program, arg_vars);
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Type {
    Tuple(Vec<TypeId>),
    Function(FunctionType),
    TypeArgument(usize, Vec<ClassId>),
    Named(String, TypeDefId, Vec<TypeId>),
}

#[derive(Debug, Clone)]
pub struct SubstitutionContext {
    type_args: BTreeMap<usize, TypeId>,
}

impl SubstitutionContext {
    pub fn new() -> SubstitutionContext {
        SubstitutionContext {
            type_args: BTreeMap::new(),
        }
    }
    pub fn get_type_id(&self, index: &usize) -> &TypeId {
        self.type_args
            .get(index)
            .expect("index not found in substitution context")
    }

    pub fn add_generic(&mut self, index: usize, type_id: TypeId) {
        let r = self.type_args.insert(index, type_id);
        assert_eq!(r, None);
    }
}
