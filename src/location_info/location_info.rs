use crate::location_info::item::Item;
use crate::location_info::item::LocationId;
use crate::location_info::location_set::LocationSet;
use crate::syntax::types::TypeSignatureId;
use crate::util::Counter;
use std::collections::BTreeMap;

pub struct TypeSignature {
    pub location: LocationSet,
}

impl TypeSignature {
    pub fn new(location: LocationSet) -> TypeSignature {
        TypeSignature { location: location }
    }
}

pub struct LocationInfo {
    items: BTreeMap<LocationId, Item>,
    type_signatures: BTreeMap<TypeSignatureId, TypeSignature>,
    id: Counter,
}

impl LocationInfo {
    pub fn new() -> LocationInfo {
        LocationInfo {
            items: BTreeMap::new(),
            type_signatures: BTreeMap::new(),
            id: Counter::new(),
        }
    }

    pub fn add_item(&mut self, item: Item) -> LocationId {
        let id = self.id.next();
        let id = LocationId { id: id };
        self.items.insert(id, item);
        id
    }

    pub fn add_type_signature(&mut self, id: TypeSignatureId, ts: TypeSignature) {
        self.type_signatures.insert(id, ts);
    }

    pub fn get_item_location(&self, id: &LocationId) -> &LocationSet {
        &self.items.get(id).expect("Item not found").location
    }

    pub fn get_type_signature_location(&self, id: &TypeSignatureId) -> &LocationSet {
        &self
            .type_signatures
            .get(id)
            .expect("TypeSignature not found")
            .location
    }
}
