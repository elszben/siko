
use crate::error::TypecheckError;
use crate::type_store::TypeStore;
use siko_ir::class::ClassMemberId;
use siko_ir::program::Program;
use std::collections::BTreeMap;

pub struct ClassMemberTypeInfo {}

pub struct ClassProcessor {
    type_store: TypeStore,
    class_member_type_info_map: BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
}

impl ClassProcessor {
    pub fn new(type_store: TypeStore) -> ClassProcessor {
        ClassProcessor {
            type_store: type_store,
            class_member_type_info_map: BTreeMap::new(),
        }
    }

    pub fn process_classes(
        mut self,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) -> (TypeStore, BTreeMap<ClassMemberId, ClassMemberTypeInfo>) {
        for (class_member_id, class_member) in &program.class_members.items {
            println!("{} = {:?}", class_member.name, class_member.type_signature);
        }

        (self.type_store, self.class_member_type_info_map)
    }
}