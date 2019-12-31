use crate::function_queue::FunctionQueue;
use crate::function_queue::FunctionQueueItem;
use crate::util::get_call_unifier;
use siko_ir::builder::Builder;
use siko_ir::class::ClassId;
use siko_ir::class::ClassMemberId;
use siko_ir::data::Record as IrRecord;
use siko_ir::data::TypeDef as IrTypeDef;
use siko_ir::data_type_info::RecordTypeInfo;
use siko_ir::expr::Expr as IrExpr;
use siko_ir::expr::ExprId as IrExprId;
use siko_ir::function::Function as IrFunction;
use siko_ir::function::FunctionId as IrFunctionId;
use siko_ir::function::FunctionInfo;
use siko_ir::function::NamedFunctionInfo;
use siko_ir::function::NamedFunctionKind;
use siko_ir::instance_resolution_cache::ResolutionResult;
use siko_ir::program::Program as IrProgram;
use siko_ir::types::Type as IrType;
use siko_location_info::location_id::LocationId;
use siko_mir::function::FunctionId as MirFunctionId;
use siko_mir::program::Program as MirProgram;

#[derive(Debug)]
pub enum DerivedClass {
    Show,
    PartialEq,
    PartialOrd,
    Ord,
}

pub fn generate_show_instance_member_for_record(
    ir_program: &mut IrProgram,
    location: LocationId,
    function_id: IrFunctionId,
    record: &IrRecord,
    record_type_info: RecordTypeInfo,
) -> (IrExprId, IrType) {
    let string_ty = ir_program.get_string_type();
    let mut builder = Builder::new(ir_program);
    let arg_ref_expr_id = builder.add_arg_ref(
        0,
        function_id,
        location,
        record_type_info.record_type.clone(),
    );
    let (bind_expr_id, values) =
        builder.add_record_pattern(arg_ref_expr_id, record, &record_type_info, location);
    let field_fmt_str_args: Vec<_> = std::iter::repeat("{{}}").take(values.len()).collect();
    let fmt_str = format!("{} {{ {} }}", record.name, field_fmt_str_args.join(", "));
    let fmt_expr = IrExpr::Formatter(fmt_str, values);
    let fmt_expr_id = builder.add_expr(fmt_expr, location, string_ty.clone());
    let items = vec![bind_expr_id, fmt_expr_id];
    let body = builder.add_expr(IrExpr::Do(items), location, string_ty.clone());
    let function_type = IrType::Function(
        Box::new(record_type_info.record_type.clone()),
        Box::new(string_ty),
    );
    (body, function_type)
}

pub fn generate_partialeq_instance_member_for_record(
    ir_program: &mut IrProgram,
    location: LocationId,
    function_id: IrFunctionId,
    record: &IrRecord,
    record_type_info: RecordTypeInfo,
    class_member_id: ClassMemberId,
) -> (IrExprId, IrType) {
    let bool_ty = ir_program.get_bool_type();
    let mut builder = Builder::new(ir_program);
    let arg_ref_expr_id_0 = builder.add_arg_ref(
        0,
        function_id,
        location,
        record_type_info.record_type.clone(),
    );
    let arg_ref_expr_id_1 = builder.add_arg_ref(
        1,
        function_id,
        location,
        record_type_info.record_type.clone(),
    );
    let (bind_expr_id_0, values_0) =
        builder.add_record_pattern(arg_ref_expr_id_0, record, &record_type_info, location);
    let (bind_expr_id_1, values_1) =
        builder.add_record_pattern(arg_ref_expr_id_1, record, &record_type_info, location);
    let mut true_branch = builder.create_bool(true, location);
    for (value_0, value_1) in values_0.iter().zip(values_1.iter()) {
        let call_expr = IrExpr::ClassFunctionCall(class_member_id, vec![*value_0, *value_1]);
        let call_expr_id = builder.add_expr(call_expr, location, bool_ty.clone());
        let false_branch = builder.create_bool(false, location);
        let if_expr = IrExpr::If(call_expr_id, true_branch, false_branch);
        true_branch = builder.add_expr(if_expr, location, bool_ty.clone());
    }
    let items = vec![bind_expr_id_0, bind_expr_id_1, true_branch];
    let body = builder.add_expr(IrExpr::Do(items), location, bool_ty.clone());
    let function_type = IrType::Function(
        Box::new(record_type_info.record_type.clone()),
        Box::new(bool_ty),
    );
    let function_type = IrType::Function(
        Box::new(record_type_info.record_type.clone()),
        Box::new(function_type),
    );
    (body, function_type)
}

pub fn generate_auto_derived_instance_member(
    class_id: ClassId,
    ir_type: &IrType,
    ir_program: &mut IrProgram,
    mir_program: &mut MirProgram,
    function_queue: &mut FunctionQueue,
    derived_class: DerivedClass,
    class_member_id: ClassMemberId,
) {
    let arg_count = match derived_class {
        DerivedClass::Show => 1,
        DerivedClass::PartialEq => 2,
        DerivedClass::PartialOrd => 2,
        DerivedClass::Ord => 2,
    };
    match ir_type {
        IrType::Named(_, typedef_id, _) => {
            let typedef = ir_program.typedefs.get(&typedef_id).clone();
            match typedef {
                IrTypeDef::Adt(adt) => {}
                IrTypeDef::Record(record) => {
                    let record_type_info = ir_program
                        .record_type_info_map
                        .get(&typedef_id)
                        .expect("Record type info not found")
                        .clone();
                    let mut location = None;
                    for derived_class in &record_type_info.derived_classes {
                        if derived_class.class_id == class_id {
                            location = Some(derived_class.location_id);
                            break;
                        }
                    }
                    let location = location.expect("Derive location not found");
                    let mut unifier = ir_program.get_unifier();
                    let r = unifier.unify(&record_type_info.record_type, &ir_type);
                    assert!(r.is_ok());
                    let function_id = ir_program.functions.get_id();
                    let (body, function_type) = match derived_class {
                        DerivedClass::Show => generate_show_instance_member_for_record(
                            ir_program,
                            location,
                            function_id,
                            &record,
                            record_type_info,
                        ),
                        DerivedClass::PartialEq => generate_partialeq_instance_member_for_record(
                            ir_program,
                            location,
                            function_id,
                            &record,
                            record_type_info,
                            class_member_id,
                        ),
                        DerivedClass::PartialOrd => unimplemented!(),
                        DerivedClass::Ord => unimplemented!(),
                    };
                    let info = NamedFunctionInfo {
                        body: Some(body),
                        kind: NamedFunctionKind::Free,
                        location_id: location,
                        type_signature: None,
                        module: record.module.clone(),
                        name: format!("{:?}_{}", derived_class, function_id.id),
                    };
                    let function_info = FunctionInfo::NamedFunction(info);
                    let function = IrFunction {
                        id: function_id,
                        arg_count: arg_count,
                        arg_locations: vec![location],
                        info: function_info,
                    };
                    ir_program.function_types.insert(function_id, function_type);
                    ir_program.functions.add_item(function_id, function);
                    let item = FunctionQueueItem::Normal(function_id, ir_program.get_unifier());
                    function_queue.insert(item, mir_program);
                }
            }
        }
        _ => unimplemented!(),
    }
}

pub fn process_class_member_call(
    arg_types: &Vec<IrType>,
    ir_program: &IrProgram,
    mir_program: &mut MirProgram,
    class_member_id: &ClassMemberId,
    expr_ty: IrType,
    function_queue: &mut FunctionQueue,
) -> MirFunctionId {
    for arg in arg_types {
        assert!(arg.is_concrete_type());
    }
    let member = ir_program.class_members.get(class_member_id);
    let (class_member_type, class_arg_ty) = ir_program
        .class_member_types
        .get(class_member_id)
        .expect("untyped class member");
    let call_unifier = get_call_unifier(
        arg_types,
        &class_member_type.remove_fixed_types(),
        &expr_ty,
        ir_program,
    );
    let class_arg = call_unifier.apply(&class_arg_ty.remove_fixed_types());
    let cache = ir_program.instance_resolution_cache.borrow();
    assert!(class_arg.is_concrete_type());
    match cache.get(member.class_id, class_arg.clone()) {
        ResolutionResult::AutoDerived => {
            let queue_item =
                FunctionQueueItem::AutoDerive(class_arg.clone(), member.class_id, *class_member_id);
            let mir_function_id = function_queue.insert(queue_item, mir_program);
            mir_function_id
        }
        ResolutionResult::UserDefined(instance_id) => {
            let instance = ir_program.instances.get(instance_id);
            let member_function_id =
                if let Some(instance_member) = instance.members.get(&member.name) {
                    instance_member.function_id
                } else {
                    member
                        .default_implementation
                        .expect("Default implementation not found")
                };
            let queue_item = FunctionQueueItem::Normal(member_function_id, call_unifier);
            let mir_function_id = function_queue.insert(queue_item, mir_program);
            mir_function_id
        }
    }
}
