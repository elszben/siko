use crate::location_info::item::LocationId;
use crate::syntax::data::Adt;
use crate::syntax::data::AdtId;
use crate::syntax::data::RecordId;
use crate::syntax::export::ExportList;
use crate::syntax::function::FunctionId;
use crate::syntax::import::Import;
use crate::syntax::import::ImportId;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ModuleId {
    pub id: usize,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub id: ModuleId,
    pub functions: Vec<FunctionId>,
    pub records: Vec<RecordId>,
    pub adts: Vec<AdtId>,
    pub imports: BTreeMap<ImportId, Import>,
    pub location_id: LocationId,
    pub export_list: ExportList,
}

impl Module {
    pub fn new(
        name: String,
        id: ModuleId,
        location_id: LocationId,
        export_list: ExportList,
    ) -> Module {
        Module {
            name: name,
            id: id,
            functions: Vec::new(),
            records: Vec::new(),
            adts: Vec::new(),
            imports: BTreeMap::new(),
            location_id: location_id,
            export_list: export_list,
        }
    }
}
