use siko_mir::types::Type;

pub enum DynTrait {
    RealCall(String, String, String),
    ArgSave(String, String, String),
}

pub struct ClosureDataDef {
    pub name: String,
    pub fields: Vec<(String, String, Type)>,
    pub traits: Vec<DynTrait>,
}
