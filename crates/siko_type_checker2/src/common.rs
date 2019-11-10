use crate::types::Type;
use crate::unifier::Unifier;
use siko_ir::class::ClassId;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::program::Program;
use siko_location_info::item::LocationId;
use std::collections::BTreeMap;

pub struct FunctionTypeInfoStore {
    function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
}

impl FunctionTypeInfoStore {
    pub fn new() -> FunctionTypeInfoStore {
        FunctionTypeInfoStore {
            function_type_info_map: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, id: FunctionId, function_type_info: FunctionTypeInfo) {
        self.function_type_info_map.insert(id, function_type_info);
    }

    pub fn get(&self, id: &FunctionId) -> &FunctionTypeInfo {
        self.function_type_info_map
            .get(id)
            .expect("Function type info not found")
    }

    pub fn get_mut(&mut self, id: &FunctionId) -> &mut FunctionTypeInfo {
        self.function_type_info_map
            .get_mut(id)
            .expect("Function type info not found")
    }

    pub fn dump(&self, program: &Program) {
        for (_, function) in &self.function_type_info_map {
            println!(
                "{} {}",
                function.displayed_name,
                function.function_type.get_resolved_type_string(program)
            );
        }
    }
}

pub struct DeriveInfo {
    pub class_id: ClassId,
    pub instance_index: usize,
}

pub struct AdtTypeInfo {
    pub adt_type: Type,
    pub variant_types: Vec<VariantTypeInfo>,
    pub derived_classes: Vec<DeriveInfo>,
}

pub struct VariantTypeInfo {
    pub item_types: Vec<(Type, LocationId)>,
}

pub struct RecordTypeInfo {
    pub record_type: Type,
    pub field_types: Vec<(Type, LocationId)>,
    pub derived_classes: Vec<DeriveInfo>,
}

pub struct FunctionTypeInfo {
    pub displayed_name: String,
    pub args: Vec<Type>,
    pub typed: bool,
    pub result: Type,
    pub function_type: Type,
    pub body: Option<ExprId>,
    pub location_id: LocationId,
}

impl FunctionTypeInfo {
    pub fn apply(&mut self, unifier: &Unifier) {
        for arg in &mut self.args {
            *arg = unifier.apply(arg);
        }
        self.result = unifier.apply(&self.result);
        self.function_type = unifier.apply(&self.function_type);
    }
}
