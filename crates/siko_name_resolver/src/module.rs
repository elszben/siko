use crate::import::ImportedItemInfo;
use crate::import::ImportedMemberInfo;
use crate::item::DataMember;
use crate::item::Item;
use siko_location_info::item::LocationId;
use siko_syntax::module::ModuleId;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub exported_items: BTreeMap<String, Item>,
    pub exported_members: BTreeMap<String, Vec<DataMember>>,
    pub imported_items: BTreeMap<String, Vec<ImportedItemInfo>>,
    pub imported_members: BTreeMap<String, Vec<ImportedMemberInfo>>,
    pub items: BTreeMap<String, Vec<Item>>,
    pub members: BTreeMap<String, Vec<DataMember>>,
    pub location_id: LocationId,
}

impl Module {
    pub fn new(id: ModuleId, name: String, location_id: LocationId) -> Module {
        Module {
            id: id,
            name: name,
            exported_items: BTreeMap::new(),
            exported_members: BTreeMap::new(),
            imported_items: BTreeMap::new(),
            imported_members: BTreeMap::new(),
            items: BTreeMap::new(),
            members: BTreeMap::new(),
            location_id: location_id,
        }
    }
}
