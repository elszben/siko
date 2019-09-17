use crate::common::InstanceTypeInfo;
use crate::error::TypecheckError;
use crate::instance_resolver::InstanceResolver;
use crate::type_store::TypeStore;
use siko_ir::class::ClassId;
use siko_ir::types::TypeInstanceResolver;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

pub struct CheckContext {
    pub instance_resolver: InstanceResolver,
    pub type_instance_resolver: Rc<RefCell<TypeInstanceResolver>>,
    pub class_names: BTreeMap<ClassId, String>,
}

impl CheckContext {
    pub fn new(type_instance_resolver: Rc<RefCell<TypeInstanceResolver>>) -> CheckContext {
        CheckContext {
            instance_resolver: InstanceResolver::new(),
            type_instance_resolver: type_instance_resolver,
            class_names: BTreeMap::new(),
        }
    }

    pub fn add_instance_info(
        &mut self,
        class_id: ClassId,
        info: InstanceTypeInfo,
        errors: &mut Vec<TypecheckError>,
        type_store: &mut TypeStore,
    ) {
        fn is_conflicting(
            first: &InstanceTypeInfo,
            second: &InstanceTypeInfo,
            type_store: &mut TypeStore,
        ) -> bool {
            let mut context = type_store.create_clone_context();
            let first = context.clone_var(first.type_var);
            let second = context.clone_var(second.type_var);
            type_store.unify(&first, &second)
        }

        let instance_infos = self
            .instance_resolver
            .instances
            .entry(class_id)
            .or_insert_with(|| Vec::new());
        let copy = instance_infos.clone();
        instance_infos.push(info.clone());
        for i in copy.iter() {
            if is_conflicting(i, &info, type_store) {
                let err = TypecheckError::ConflictingInstances(i.location_id, info.location_id);
                errors.push(err);
            }
        }
    }

    pub fn finished_instance_checks(&mut self) {
        // during instance conflict checks the instance resolver
        // will tell that every type has an instance for every type class
        // to ignore instance constraints
        // e.q. these instance are conflicting even if Int does
        // not implement Bar
        // instance (Bar a) => Foo a
        // instance Foo Int
        self.instance_resolver.hide_deps = false;
    }
}
