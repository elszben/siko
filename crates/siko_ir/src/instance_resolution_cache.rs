use crate::class::ClassId;
use crate::class::InstanceId;
use crate::types::Type;
use std::collections::BTreeMap;

#[derive(Debug)]
pub enum ResolutionResult {
    UserDefined(InstanceId),
    AutoDerived,
}

pub struct InstanceResolutionCache {
    cache: BTreeMap<(ClassId, Type), ResolutionResult>,
}

impl InstanceResolutionCache {
    pub fn new() -> InstanceResolutionCache {
        InstanceResolutionCache {
            cache: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, class_id: ClassId, ty: Type, result: ResolutionResult) {
        self.cache.insert((class_id, ty), result);
    }

    pub fn get(&self, class_id: ClassId, ty: Type) -> &ResolutionResult {
        self.cache
            .get(&(class_id, ty))
            .expect("Instance resolution result not found")
    }
}
