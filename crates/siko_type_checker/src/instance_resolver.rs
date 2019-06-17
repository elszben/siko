use crate::common::InstanceTypeInfo;
use crate::type_store::TypeStore;
use crate::type_variable::TypeVariable;
use siko_ir::class::ClassId;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct InstanceResolver {
    pub instances: BTreeMap<ClassId, Vec<InstanceTypeInfo>>,
}

impl InstanceResolver {
    pub fn new() -> InstanceResolver {
        InstanceResolver {
            instances: BTreeMap::new(),
        }
    }

    pub fn has_class_instance(
        &self,
        var: &TypeVariable,
        class_id: &ClassId,
        type_store: &mut TypeStore,
    ) -> bool {
        if let Some(class_instances) = self.instances.get(class_id) {
            for instance in class_instances {
                let mut context = type_store.create_clone_context();
                let instance_var = context.clone_var(instance.type_var);
                let second_instance_var = context.clone_var(instance.type_var);
                let input_var = context.clone_var(*var);
                if type_store.unify(&input_var, &instance_var) {
                    type_store.unify(&second_instance_var, var);
                    return true;
                }
            }
            false
        } else {
            false
        }
    }
}
