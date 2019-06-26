use crate::class::ClassId;
use crate::class::InstanceId;
use crate::data::AdtId;
use crate::data::RecordId;
use crate::export_import::EIList;
use crate::function::FunctionId;
use crate::function::FunctionTypeId;
use crate::import::ImportId;
use siko_location_info::item::LocationId;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub struct ModuleId {
    pub id: usize,
}

impl From<usize> for ModuleId {
    fn from(id: usize) -> ModuleId {
        ModuleId { id: id }
    }
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub id: ModuleId,
    pub functions: Vec<FunctionId>,
    pub function_types: Vec<FunctionTypeId>,
    pub records: Vec<RecordId>,
    pub adts: Vec<AdtId>,
    pub classes: Vec<ClassId>,
    pub instances: Vec<InstanceId>,
    pub imports: Vec<ImportId>,
    pub location_id: LocationId,
    pub export_list: EIList,
}

impl Module {
    pub fn new(name: String, id: ModuleId, location_id: LocationId, export_list: EIList) -> Module {
        Module {
            name: name,
            id: id,
            functions: Vec::new(),
            function_types: Vec::new(),
            records: Vec::new(),
            adts: Vec::new(),
            classes: Vec::new(),
            instances: Vec::new(),
            imports: Vec::new(),
            location_id: location_id,
            export_list: export_list,
        }
    }
}
