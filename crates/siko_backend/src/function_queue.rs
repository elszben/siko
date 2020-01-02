use crate::class_member_processor::generate_auto_derived_instance_member;
use crate::class_member_processor::DerivedClass;
use crate::function_processor::process_function;
use crate::typedef_store::TypeDefStore;
use siko_ir::class::ClassId;
use siko_ir::class::ClassMemberId;
use siko_ir::function::FunctionId as IrFunctionId;
use siko_ir::program::Program as IrProgram;
use siko_ir::types::Type as IrType;
use siko_mir::function::FunctionId as MirFunctionId;
use siko_mir::program::Program as MirProgram;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FunctionQueueItem {
    Normal(IrFunctionId, Vec<IrType>, IrType),
    AutoDerive(IrType, ClassId, ClassMemberId),
}

pub struct FunctionQueue {
    pending: Vec<(FunctionQueueItem, MirFunctionId)>,
    processed: BTreeMap<FunctionQueueItem, MirFunctionId>,
}

impl FunctionQueue {
    pub fn new() -> FunctionQueue {
        FunctionQueue {
            pending: Vec::new(),
            processed: BTreeMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        item: FunctionQueueItem,
        mir_program: &mut MirProgram,
    ) -> MirFunctionId {
        let mut pending = false;
        let mir_function_id = self.processed.entry(item.clone()).or_insert_with(|| {
            pending = true;
            mir_program.functions.get_id()
        });
        if pending {
            self.pending.push((item, *mir_function_id));
        }
        *mir_function_id
    }

    pub fn process_items(
        &mut self,
        ir_program: &mut IrProgram,
        mir_program: &mut MirProgram,
        typedef_store: &mut TypeDefStore,
    ) {
        while !self.pending.is_empty() {
            if let Some((item, mir_function_id)) = self.pending.pop() {
                match item {
                    FunctionQueueItem::Normal(function_id, arg_types, result_ty) => {
                        process_function(
                            &function_id,
                            mir_function_id,
                            ir_program,
                            mir_program,
                            arg_types,
                            result_ty,
                            self,
                            typedef_store,
                        );
                    }
                    FunctionQueueItem::AutoDerive(ir_type, class_id, class_member_id) => {
                        let class = ir_program.classes.get(&class_id);
                        match (class.module.as_ref(), class.name.as_ref()) {
                            ("Std.Ops", "Show") => {
                                generate_auto_derived_instance_member(
                                    class_id,
                                    &ir_type,
                                    ir_program,
                                    DerivedClass::Show,
                                    class_member_id,
                                );
                            }
                            ("Std.Ops", "PartialEq") => {
                                generate_auto_derived_instance_member(
                                    class_id,
                                    &ir_type,
                                    ir_program,
                                    DerivedClass::PartialEq,
                                    class_member_id,
                                );
                            }
                            ("Std.Ops", "PartialOrd") => {
                                generate_auto_derived_instance_member(
                                    class_id,
                                    &ir_type,
                                    ir_program,
                                    DerivedClass::PartialOrd,
                                    class_member_id,
                                );
                            }
                            ("Std.Ops", "Ord") => {
                                generate_auto_derived_instance_member(
                                    class_id,
                                    &ir_type,
                                    ir_program,
                                    DerivedClass::Ord,
                                    class_member_id,
                                );
                            }
                            _ => panic!(
                                "Auto derive of {}/{} is not implemented",
                                class.module, class.name
                            ),
                        }
                    }
                }
            }
        }
    }
}