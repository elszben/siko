use crate::data::TypeDefId;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Type {
    Tuple(Vec<Type>),
    Named(String, TypeDefId),
    Function(Box<Type>, Box<Type>),
}

impl Type {
    pub fn as_string(&self) -> String {
        return self.as_string_internal(false);
    }

    fn as_string_internal(&self, need_parens: bool) -> String {
        match self {
            Type::Tuple(items) => {
                let ss: Vec<_> = items
                    .iter()
                    .map(|item| item.as_string_internal(false))
                    .collect();
                format!("({})", ss.join(", "))
            }
            Type::Function(from, to) => {
                let from_str = from.as_string_internal(true);
                let to_str = to.as_string_internal(true);
                let func_type_str = format!("{} -> {}", from_str, to_str);
                if need_parens {
                    format!("({})", func_type_str)
                } else {
                    func_type_str
                }
            }
            Type::Named(name, _) => {
                if need_parens {
                    format!("({})", name,)
                } else {
                    format!("{}", name)
                }
            }
        }
    }
}
