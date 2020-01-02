use crate::expr_processor::process_expr;
use crate::function_queue::FunctionQueue;
use crate::type_processor::process_type;
use crate::typedef_store::TypeDefStore;
use crate::util::get_call_unifier;
use crate::util::preprocess_ir;
use siko_ir::function::FunctionId as IrFunctionId;
use siko_ir::function::FunctionInfo;
use siko_ir::program::Program as IrProgram;
use siko_ir::types::Type;
use siko_mir::function::Function as MirFunction;
use siko_mir::function::FunctionId as MirFunctionId;
use siko_mir::function::FunctionInfo as MirFunctionInfo;
use siko_mir::program::Program as MirProgram;

pub fn process_function(
    ir_function_id: &IrFunctionId,
    mir_function_id: MirFunctionId,
    ir_program: &mut IrProgram,
    mir_program: &mut MirProgram,
    arg_types: Vec<Type>,
    result_ty: Type,
    function_queue: &mut FunctionQueue,
    typedef_store: &mut TypeDefStore,
) {
    let mut function_type = ir_program
        .get_function_type(ir_function_id)
        .remove_fixed_types();
    let call_unifier = get_call_unifier(&arg_types, &function_type, &result_ty, ir_program);
    function_type.apply(&call_unifier);
    let mir_function_type = process_type(&function_type, typedef_store, ir_program, mir_program);
    let function = ir_program.functions.get(ir_function_id).clone();
    match &function.info {
        FunctionInfo::NamedFunction(info) => {
            let mir_function_info = if let Some(body) = info.body {
                preprocess_ir(body, ir_program);
                let mir_expr_id = process_expr(
                    &body,
                    ir_program,
                    mir_program,
                    &call_unifier,
                    function_queue,
                    typedef_store,
                );
                MirFunctionInfo::Normal(mir_expr_id)
            } else {
                MirFunctionInfo::Extern
            };
            let mir_function = MirFunction {
                name: info.name.clone(),
                module: info.module.clone(),
                info: mir_function_info,
                function_type: mir_function_type,
            };
            mir_program
                .functions
                .add_item(mir_function_id, mir_function);
        }
        FunctionInfo::Lambda(info) => {
            preprocess_ir(info.body, ir_program);
            let mir_body = process_expr(
                &info.body,
                ir_program,
                mir_program,
                &call_unifier,
                function_queue,
                typedef_store,
            );
            let lambda_name = format!("{}", info);
            let lambda_name = lambda_name.replace("/", "_");
            let lambda_name = lambda_name.replace(".", "_");
            let lambda_name = lambda_name.replace("#", "_");
            let mir_function = MirFunction {
                name: lambda_name,
                module: info.module.clone(),
                info: MirFunctionInfo::Normal(mir_body),
                function_type: mir_function_type,
            };
            mir_program
                .functions
                .add_item(mir_function_id, mir_function);
        }
        FunctionInfo::VariantConstructor(info) => {
            let adt = ir_program.typedefs.get(&info.type_id).get_adt();
            let variant = &adt.variants[info.index];
            let module = adt.module.clone();
            let result_ty = function_type.get_result_type(function.arg_count);
            let mir_typedef_id = typedef_store.add_type(result_ty, ir_program, mir_program);
            let mir_function = MirFunction {
                name: format!("{}_{}_ctor{}", adt.name, variant.name, info.index),
                module: module,
                info: MirFunctionInfo::VariantConstructor(mir_typedef_id, info.index),
                function_type: mir_function_type,
            };
            mir_program
                .functions
                .add_item(mir_function_id, mir_function);
        }
        _ => {}
    }
}