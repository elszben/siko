use crate::substitution::Error;
use crate::substitution::Substitution;
use crate::types::Type;

pub struct Unifier {
    substitution: Substitution,
}

impl Unifier {
    pub fn new() -> Unifier {
        Unifier {
            substitution: Substitution::empty(),
        }
    }

    pub fn unify(&mut self, type1: &Type, type2: &Type) -> Result<(), Error> {
        match (type1, type2) {
            (Type::Named(_, id1, items1), Type::Named(_, id2, items2)) => {
                if id1 == id2 {
                    assert_eq!(items1.len(), items2.len());
                    for (item1, item2) in items1.iter().zip(items2.iter()) {
                        self.unify(item1, item2)?;
                    }
                    Ok(())
                } else {
                    return Err(Error::Fail);
                }
            }
            (Type::Var(index, constraints), _) => {
                for c in constraints {
                    self.substitution.add_constraint(*c, type2.clone());
                }
                return self.substitution.add(*index, type2);
            }
            (_, Type::Var(index, constraints)) => {
                for c in constraints {
                    self.substitution.add_constraint(*c, type1.clone());
                }
                return self.substitution.add(*index, type1);
            }
            (Type::Tuple(items1), Type::Tuple(items2)) => {
                if items1.len() != items2.len() {
                    return Err(Error::Fail);
                }
                for (item1, item2) in items1.iter().zip(items2.iter()) {
                    self.unify(item1, item2)?;
                }
                Ok(())
            }
            (Type::Function(from1, to1), Type::Function(from2, to2)) => {
                self.unify(from1, from2)?;
                self.unify(to1, to2)?;
                Ok(())
            }
            _ => return Err(Error::Fail),
        }
    }

    pub fn apply(&self, ty: &Type) -> Type {
        self.substitution.apply(ty)
    }
}
