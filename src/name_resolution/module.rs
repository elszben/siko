use crate::location_info::item::LocationId;
use crate::name_resolution::export::ExportedDataMember;
use crate::name_resolution::export::ExportedItem;
use crate::name_resolution::import::ImportItemInfo;
use crate::name_resolution::import::ImportMemberInfo;
use crate::name_resolution::item::Item;
use crate::syntax::item_path::ItemPath;
use crate::syntax::module::ModuleId;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Module {
    pub id: ModuleId,
    pub name: ItemPath,
    pub exported_items: BTreeMap<String, ExportedItem>,
    pub exported_members: BTreeMap<String, Vec<ExportedDataMember>>,
    pub imported_items: BTreeMap<String, Vec<ImportItemInfo>>,
    pub imported_members: BTreeMap<String, Vec<ImportMemberInfo>>,
    pub items: BTreeMap<String, Vec<Item>>,
    pub location_id: LocationId,
}

impl Module {
    pub fn new(id: ModuleId, name: ItemPath, location_id: LocationId) -> Module {
        Module {
            id: id,
            name: name,
            exported_items: BTreeMap::new(),
            exported_members: BTreeMap::new(),
            imported_items: BTreeMap::new(),
            imported_members: BTreeMap::new(),
            items: BTreeMap::new(),
            location_id: location_id,
        }
    }
}
