use crate::data::TypeDefId;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Type {
    Named(TypeDefId),
    Function(Box<Type>, Box<Type>),
}

impl Type {
    pub fn get_args(&self, args: &mut Vec<Type>) {
        match self {
            Type::Named(..) => {}
            Type::Function(from, to) => {
                args.push(*from.clone());
                to.get_args(args);
            }
        }
    }

    pub fn get_result_type(&self, arg_count: usize) -> Type {
        match self {
            Type::Named(..) => self.clone(),
            Type::Function(_, to) => {
                if arg_count == 1 {
                    *to.clone()
                } else {
                    if arg_count == 0 {
                        self.clone()
                    } else {
                        to.get_result_type(arg_count - 1)
                    }
                }
            }
        }
    }

    pub fn get_typedef_id(&self) -> TypeDefId {
        match self {
            Type::Named(id) => *id,
            Type::Function(_, _) => unreachable!(),
        }
    }
}
