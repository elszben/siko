use crate::class::ClassId;
use crate::class::InstanceId;
use crate::data::TypeDefId;
use crate::function::FunctionId;
use crate::program::Program;
use crate::type_signature::TypeSignatureId;
use siko_location_info::item::LocationId;
use std::collections::BTreeMap;
use std::fmt;

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

impl ConcreteType {
    pub fn get_func_type(self, arg_count: usize) -> ConcreteType {
        if arg_count == 0 {
            self
        } else {
            match self {
                ConcreteType::Function(_, to) => to.get_func_type(arg_count - 1),
                _ => {
                    println!("{} with {} args", self, arg_count);
                    assert_eq!(arg_count, 0);
                    self
                }
            }
        }
    }

    pub fn get_type_args(&self) -> Vec<ConcreteType> {
        match self {
            ConcreteType::Named(_, _, items) => items.clone(),
            _ => unreachable!(),
        }
    }
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

    pub fn get_arg_and_return_types(
        &self,
        program: &Program,
        arg_vars: &mut Vec<TypeId>,
        arg_count: usize,
    ) -> TypeId {
        if arg_count == 1 {
            arg_vars.push(self.from);
            self.to
        } else {
            if let Type::Function(to_func_type) =
                program.types.get(&self.to).expect("Type not found")
            {
                arg_vars.push(self.from);
                to_func_type.get_arg_and_return_types(program, arg_vars, arg_count - 1)
            } else {
                assert_eq!(arg_count, 0);
                self.to
            }
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
    type_args: BTreeMap<usize, ConcreteType>,
}

impl SubstitutionContext {
    pub fn new() -> SubstitutionContext {
        SubstitutionContext {
            type_args: BTreeMap::new(),
        }
    }
    pub fn get_type_id(&self, index: &usize) -> &ConcreteType {
        self.type_args
            .get(index)
            .expect("index not found in substitution context")
    }

    pub fn add_generic(&mut self, index: usize, concrete_ty: ConcreteType) {
        match self.type_args.get(&index) {
            Some(ty) => {
                assert_eq!(*ty, concrete_ty);
            }
            None => {
                self.type_args.insert(index, concrete_ty);
            }
        }
    }
}