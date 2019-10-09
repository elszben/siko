use crate::common::InstanceTypeInfo;
use crate::type_store::TypeStore;
use crate::type_variable::TypeVariable;
use siko_ir::class::ClassId;
use siko_ir::class::InstanceId;
use siko_ir::types::ConcreteType;
use siko_ir::types::TypeInstanceResolver;
use siko_util::ElapsedTimeMeasureCollector;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Instant;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ResolutionResult {
    Inconclusive,
    Definite(InstanceId),
    No,
}

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

    fn has_class_instance_inner(
        &self,
        var: &TypeVariable,
        class_id: &ClassId,
        type_store: &mut TypeStore,
        type_instance_resolver: Rc<RefCell<TypeInstanceResolver>>,
        concrete_type: Option<ConcreteType>,
    ) -> ResolutionResult {
        if self.hide_deps {
            return ResolutionResult::Inconclusive;
        }
        if let Some(class_instances) = self.instances.get(class_id) {
            for instance in class_instances {
                let mut context = type_store.create_clone_context();
                let test_instance_var = context.clone_var(instance.type_var);
                let second_instance_var = context.clone_var(instance.type_var);
                let test_input_var = context.clone_var(*var);
                if type_store.unify(&test_input_var, &test_instance_var) {
                    type_store.unify(&second_instance_var, var);
                    if let Some(concrete_type) = concrete_type {
                        let mut r = type_instance_resolver.borrow_mut();
                        r.add(*class_id, concrete_type, instance.instance_id);
                    }
                    return ResolutionResult::Definite(instance.instance_id);
                }
            }
            ResolutionResult::No
        } else {
            ResolutionResult::No
        }
    }

    pub fn has_class_instance(
        &self,
        var: &TypeVariable,
        class_id: &ClassId,
        type_store: &mut TypeStore,
        type_instance_resolver: Rc<RefCell<TypeInstanceResolver>>,
        concrete_type: Option<ConcreteType>,
    ) -> ResolutionResult {
        let start = Instant::now();
        let r = self.has_class_instance_inner(
            var,
            class_id,
            type_store,
            type_instance_resolver,
            concrete_type,
        );
        let end = Instant::now();
        ElapsedTimeMeasureCollector::add_instance_resolver_time(end - start);
        r
    }
}
