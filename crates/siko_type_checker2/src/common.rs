use crate::type_var_generator::TypeVarGenerator;
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

#[derive(Clone, Debug)]
pub struct DeriveInfo {
    pub class_id: ClassId,
    pub instance_index: usize,
}

#[derive(Clone)]
pub struct AdtTypeInfo {
    pub adt_type: Type,
    pub variant_types: Vec<VariantTypeInfo>,
    pub derived_classes: Vec<DeriveInfo>,
}

impl AdtTypeInfo {
    pub fn apply(&mut self, unifier: &Unifier) -> bool {
        let mut changed = false;
        for variant_type in &mut self.variant_types {
            changed = variant_type.apply(unifier) || changed;
        }
        changed = self.adt_type.apply(unifier) || changed;
        changed
    }

    pub fn duplicate(&self, type_var_generator: &mut TypeVarGenerator) -> AdtTypeInfo {
        let mut arg_map = BTreeMap::new();
        AdtTypeInfo {
            adt_type: self.adt_type.duplicate(&mut arg_map, type_var_generator),
            variant_types: self
                .variant_types
                .iter()
                .map(|ty| ty.duplicate(&mut arg_map, type_var_generator))
                .collect(),
            derived_classes: self.derived_classes.clone(),
        }
    }
}

#[derive(Clone)]
pub struct VariantTypeInfo {
    pub item_types: Vec<(Type, LocationId)>,
}

impl VariantTypeInfo {
    pub fn apply(&mut self, unifier: &Unifier) -> bool {
        let mut changed = false;
        for item_type in &mut self.item_types {
            changed = item_type.0.apply(unifier) || changed;
        }
        changed
    }

    pub fn duplicate(
        &self,
        arg_map: &mut BTreeMap<usize, usize>,
        type_var_generator: &mut TypeVarGenerator,
    ) -> VariantTypeInfo {
        VariantTypeInfo {
            item_types: self
                .item_types
                .iter()
                .map(|(ty, location)| (ty.duplicate(arg_map, type_var_generator), *location))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecordTypeInfo {
    pub record_type: Type,
    pub field_types: Vec<(Type, LocationId)>,
    pub derived_classes: Vec<DeriveInfo>,
}

impl RecordTypeInfo {
    pub fn apply(&mut self, unifier: &Unifier) -> bool {
        let mut changed = false;
        for field_type in &mut self.field_types {
            changed = field_type.0.apply(unifier) || changed;
        }
        changed = self.record_type.apply(unifier) || changed;
        changed
    }

    pub fn duplicate(&self, type_var_generator: &mut TypeVarGenerator) -> RecordTypeInfo {
        let mut arg_map = BTreeMap::new();
        RecordTypeInfo {
            record_type: self.record_type.duplicate(&mut arg_map, type_var_generator),
            field_types: self
                .field_types
                .iter()
                .map(|(ty, location)| (ty.duplicate(&mut arg_map, type_var_generator), *location))
                .collect(),
            derived_classes: self.derived_classes.clone(),
        }
    }
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

pub struct ClassMemberTypeInfo {
    pub ty: Type,
}
