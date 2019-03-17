use crate::location_info::item::LocationId;
use crate::name_resolution::export::ExportedItem;
use crate::name_resolution::export::ExportedType;
use crate::name_resolution::import::ImportItemInfo;
use crate::name_resolution::import::ImportTypeInfo;
use crate::name_resolution::item::Item;
use crate::name_resolution::item::Type;
use crate::syntax::item_path::ItemPath;
use crate::syntax::module::ModuleId;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Module {
    pub id: ModuleId,
    pub name: ItemPath,
    pub exported_items: BTreeMap<String, ExportedItem>,
    pub exported_types: BTreeMap<String, ExportedType>,
    pub imported_items: BTreeMap<String, Vec<ImportItemInfo>>,
    pub imported_types: BTreeMap<String, Vec<ImportTypeInfo>>,
    pub items: BTreeMap<String, Vec<Item>>,
    pub types: BTreeMap<String, Vec<Type>>,
    pub location_id: LocationId,
}

impl Module {
    pub fn new(id: ModuleId, name: ItemPath, location_id: LocationId) -> Module {
        Module {
            id: id,
            name: name,
            exported_items: BTreeMap::new(),
            exported_types: BTreeMap::new(),
            imported_items: BTreeMap::new(),
            imported_types: BTreeMap::new(),
            items: BTreeMap::new(),
            types: BTreeMap::new(),
            location_id: location_id,
        }
    }
}
