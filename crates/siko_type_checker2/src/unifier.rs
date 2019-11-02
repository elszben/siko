use crate::substitution::Constraint;
use crate::substitution::Error;
use crate::substitution::Substitution;
use crate::typechecker::TypeVarGenerator;
use crate::types::Type;
use siko_ir::class::ClassId;

pub struct Unifier {
    type_var_generator: TypeVarGenerator,
    substitution: Substitution,
}

impl Unifier {
    pub fn new(type_var_generator: TypeVarGenerator) -> Unifier {
        Unifier {
            type_var_generator: type_var_generator,
            substitution: Substitution::empty(),
        }
    }

    pub fn unify(&mut self, type1: &Type, type2: &Type) -> Result<(), Error> {
        let type1 = self.apply(type1);
        let type2 = self.apply(type2);
        match (&type1, &type2) {
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
            (Type::Var(index1, constraints1), Type::Var(index2, constraints2)) => {
                let mut merged = constraints1.clone();
                merged.extend(constraints2);
                merged.sort();
                merged.dedup();
                let merged_type = Type::Var(self.type_var_generator.get_new_index(), merged);
                self.substitution.add(*index1, &merged_type)?;
                return self.substitution.add(*index2, &merged_type);
            }
            (Type::FixedTypeArg(_, index1, _), Type::FixedTypeArg(_, index2, _)) => {
                if index1 == index2 {
                    Ok(())
                } else {
                    return Err(Error::Fail);
                }
            }
            (Type::FixedTypeArg(_, _, constraints1), Type::Var(index2, constraints2)) => {
                for c in constraints2 {
                    if !constraints1.contains(c) {
                        return Err(Error::Fail);
                    }
                }
                return self.substitution.add(*index2, &type1);
            }
            (Type::Var(index1, constraints1), Type::FixedTypeArg(_, _, constraints2)) => {
                for c in constraints1 {
                    if !constraints2.contains(c) {
                        return Err(Error::Fail);
                    }
                }
                return self.substitution.add(*index1, &type2);
            }
            (Type::Var(index, constraints), type2) => {
                for c in constraints {
                    self.substitution.add_constraint(*c, type2.clone());
                }
                return self.substitution.add(*index, &type2);
            }
            (type1, Type::Var(index, constraints)) => {
                for c in constraints {
                    self.substitution.add_constraint(*c, type1.clone());
                }
                return self.substitution.add(*index, &type1);
            }
            (Type::FixedTypeArg(_, index, constraints), type2) => {
                for c in constraints {
                    self.substitution.add_constraint(*c, type2.clone());
                }
                return self.substitution.add(*index, &type2);
            }
            (type1, Type::FixedTypeArg(_, index, constraints)) => {
                for c in constraints {
                    self.substitution.add_constraint(*c, type1.clone());
                }
                return self.substitution.add(*index, &type1);
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
                self.unify(&from1, &from2)?;
                self.unify(&to1, &to2)?;
                Ok(())
            }
            _ => return Err(Error::Fail),
        }
    }

    pub fn apply(&self, ty: &Type) -> Type {
        self.substitution.apply(ty)
    }

    pub fn dump(&self) {
        self.substitution.dump();
    }

    pub fn get_constraints(&self) -> Vec<Constraint> {
        self.substitution.get_constraints()
    }
}
