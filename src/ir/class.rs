use std::fmt;

#[derive(Debug, Clone)]
pub struct Class {
    pub id: ClassId,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ClassId {
    pub id: usize,
}

impl fmt::Display for ClassId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.id)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ClassMemberId {
    pub id: usize,
}

impl fmt::Display for ClassMemberId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}", self.id)
    }
}
