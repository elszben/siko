use crate::location_info::location_set::LocationSet;

pub struct Item {
    pub location: LocationSet,
}

impl Item {
    pub fn new(location: LocationSet) -> Item {
        Item { location: location }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct LocationId {
    pub id: usize,
}
