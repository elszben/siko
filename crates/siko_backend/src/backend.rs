use crate::function_queue::FunctionQueue;
use crate::function_queue::FunctionQueueItem;
use crate::typedef_store::TypeDefStore;
use siko_ir::program::Program as IrProgram;
use siko_ir::types::Type;
use siko_mir::program::Program as MirProgram;

pub struct Backend {}

impl Backend {
    pub fn compile(ir_program: &mut IrProgram) -> Result<MirProgram, ()> {
        let mut mir_program = MirProgram::new();
        let mut function_queue = FunctionQueue::new();
        let mut typedef_store = TypeDefStore::new();
        let main_id = ir_program.get_main().expect("Main not found");
        function_queue.insert(
            FunctionQueueItem::Normal(main_id, vec![], Type::Tuple(vec![])),
            &mut mir_program,
        );
        function_queue.process_items(ir_program, &mut mir_program, &mut typedef_store);
        Ok(mir_program)
    }
}
