use crate::location_info::item::LocationId;
use crate::syntax::data::Adt;
use crate::syntax::data::AdtId;
use crate::syntax::data::Record;
use crate::syntax::data::RecordId;
use crate::syntax::export::ExportList;
use crate::syntax::function::Function;
use crate::syntax::function::FunctionId;
use crate::syntax::import::Import;
use crate::syntax::import::ImportId;
use crate::syntax::item_path::ItemPath;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ModuleId {
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: ItemPath,
    pub id: ModuleId,
    pub functions: BTreeMap<FunctionId, Function>,
    pub imports: BTreeMap<ImportId, Import>,
    pub location_id: LocationId,
    pub export_list: ExportList,
    pub records: BTreeMap<RecordId, Record>,
    pub adts: BTreeMap<AdtId, Adt>,
}

impl Module {
    pub fn new(
        name: ItemPath,
        id: ModuleId,
        location_id: LocationId,
        export_list: ExportList,
    ) -> Module {
        Module {
            name: name,
            id: id,
            functions: BTreeMap::new(),
            imports: BTreeMap::new(),
            location_id: location_id,
            export_list: export_list,
            records: BTreeMap::new(),
            adts: BTreeMap::new(),
        }
    }
}
