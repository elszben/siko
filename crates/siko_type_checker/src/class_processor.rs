use crate::common::ClassMemberTypeInfo;
use crate::error::TypecheckError;
use crate::type_processor::process_type_signature;
use crate::type_store::TypeStore;
use crate::type_variable::TypeVariable;
use siko_ir::class::ClassId;
use siko_ir::class::ClassMemberId;
use siko_ir::class::InstanceId;
use siko_ir::program::Program;
use siko_location_info::item::LocationId;
use std::collections::BTreeMap;

pub struct InstanceTypeInfo {
    pub instance_id: InstanceId,
    pub type_var: TypeVariable,
    pub location_id: LocationId,
}

impl InstanceTypeInfo {
    pub fn new(
        instance_id: InstanceId,
        type_var: TypeVariable,
        location_id: LocationId,
    ) -> InstanceTypeInfo {
        InstanceTypeInfo {
            instance_id: instance_id,
            type_var: type_var,
            location_id: location_id,
        }
    }
}

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

pub struct ClassProcessor {
    type_store: TypeStore,
    class_member_type_info_map: BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
    instances: BTreeMap<ClassId, Vec<InstanceTypeInfo>>,
}

impl ClassProcessor {
    pub fn new(type_store: TypeStore) -> ClassProcessor {
        ClassProcessor {
            type_store: type_store,
            class_member_type_info_map: BTreeMap::new(),
            instances: BTreeMap::new(),
        }
    }

    pub fn process_classes(
        mut self,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) -> (TypeStore, BTreeMap<ClassMemberId, ClassMemberTypeInfo>) {
        for (class_id, class) in &program.classes.items {
            self.type_store
                .class_names
                .insert(*class_id, class.name.clone());
        }
        for (class_member_id, class_member) in &program.class_members.items {
            //println!("{} = {:?}", class_member.name, class_member.type_signature);
            let mut arg_map = BTreeMap::new();
            let var = process_type_signature(
                &mut self.type_store,
                &class_member.type_signature,
                program,
                &mut arg_map,
            );

            let info = ClassMemberTypeInfo {
                member_type_var: var,
            };
            self.class_member_type_info_map
                .insert(*class_member_id, info);
            //let type_str = self.type_store.get_resolved_type_string(&var, program);
            //println!("{}", type_str);
        }

        for (instance_id, instance) in &program.instances.items {
            let mut arg_map = BTreeMap::new();
            let var = process_type_signature(
                &mut self.type_store,
                &instance.type_signature,
                program,
                &mut arg_map,
            );
            let info = InstanceTypeInfo::new(*instance_id, var, instance.location_id);
            let instance_infos = self
                .instances
                .entry(instance.class_id)
                .or_insert_with(|| Vec::new());
            for i in instance_infos.iter() {
                if is_conflicting(i, &info, &mut self.type_store) {
                    let err = TypecheckError::ConflictingInstances(i.location_id, info.location_id);
                    errors.push(err);
                }
            }
            instance_infos.push(info);
        }

        (self.type_store, self.class_member_type_info_map)
    }
}
