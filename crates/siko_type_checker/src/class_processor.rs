use crate::common::ClassMemberTypeInfo;
use crate::common::InstanceTypeInfo;
use crate::error::TypecheckError;
use crate::type_processor::process_type_signature;
use crate::type_store::TypeStore;
use siko_ir::class::ClassMemberId;
use siko_ir::program::Program;
use std::collections::BTreeMap;
use crate::check_context::CheckContext;
use std::cell::RefCell;
use std::rc::Rc;

pub struct ClassProcessor {
    type_store: TypeStore,
    check_context: Rc<RefCell<CheckContext>>,
    class_member_type_info_map: BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
}

impl ClassProcessor {
    pub fn new(type_store: TypeStore, check_context: Rc<RefCell<CheckContext>>) -> ClassProcessor {
        ClassProcessor {
            type_store: type_store,
            check_context:check_context,
            class_member_type_info_map: BTreeMap::new(),
            
        }
    }

    pub fn process_classes(
        mut self,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) -> (TypeStore, BTreeMap<ClassMemberId, ClassMemberTypeInfo>) {
        let mut check_context = self.check_context.borrow_mut();
        for (class_id, class) in &program.classes.items {
            check_context
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
            check_context
                .add_instance_info(instance.class_id, info, errors, &mut self.type_store);
        }

        check_context.finished_instance_checks();

        (self.type_store, self.class_member_type_info_map)
    }
}
