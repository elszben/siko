use crate::common::InstanceTypeInfo;
use crate::type_store::TypeStore;
use crate::type_variable::TypeVariable;
use siko_ir::class::ClassId;
use siko_ir::types::ConcreteType;
use siko_ir::types::TypeInstanceResolver;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

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
        type_instance_resolver: Rc<RefCell<TypeInstanceResolver>>,
        concrete_type: Option<ConcreteType>,
    ) -> bool {
        if self.hide_deps {
            return true;
        }
        if let Some(class_instances) = self.instances.get(class_id) {
            for instance in class_instances {
                let mut context = type_store.create_clone_context();
                let instance_var = context.clone_var(instance.type_var);
                let second_instance_var = context.clone_var(instance.type_var);
                let input_var = context.clone_var(*var);
                if type_store.unify(&input_var, &instance_var) {
                    type_store.unify(&second_instance_var, var);
                    if let Some(concrete_type) = concrete_type {
                        let mut r = type_instance_resolver.borrow_mut();
                        r.add(*class_id, concrete_type, instance.instance_id);
                    }
                    return true;
                }
            }
            false
        } else {
            false
        }
    }
}
