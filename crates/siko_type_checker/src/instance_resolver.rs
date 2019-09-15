use crate::common::InstanceTypeInfo;
use crate::type_store::TypeStore;
use crate::type_variable::TypeVariable;
use siko_ir::class::ClassId;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct InstanceResolver {
    pub instances: BTreeMap<ClassId, Vec<InstanceTypeInfo>>,
    pub hide_deps: bool,
}

impl InstanceResolver {
    pub fn new() -> InstanceResolver {
        InstanceResolver {
            instances: BTreeMap::new(),
            hide_deps: true,
        }
    }

    pub fn has_class_instance(
        &self,
        var: &TypeVariable,
        class_id: &ClassId,
        type_store: &mut TypeStore,
    ) -> bool {
        if self.hide_deps {
            return true;
        }
        if let Some(class_instances) = self.instances.get(class_id) {
            let mut count = 0;
            for instance in class_instances {
                let mut context = type_store.create_clone_context();
                let instance_var = context.clone_var(instance.type_var);
                let second_instance_var = context.clone_var(instance.type_var);
                let input_var = context.clone_var(*var);
                if type_store.unify(&input_var, &instance_var) {
                    type_store.unify(&second_instance_var, var);
                    count += 1;
                    println!("Matching instance {}", instance.instance_id);
                }
            }
            assert!(count < 2);
            count == 1
        } else {
            false
        }
    }
}
