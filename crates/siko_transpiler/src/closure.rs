pub enum DynTrait {
    RealCall(String, String, String),
    ArgSave(String, String, String),
}

pub struct ClosureDataDef {
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub traits: Vec<DynTrait>,
}
