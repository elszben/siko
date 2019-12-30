use siko_ir::class::ClassMemberId;
use siko_ir::data::TypeDef as IrTypeDef;
use siko_ir::expr::Expr as IrExpr;
use siko_ir::expr::ExprId as IrExprId;
use siko_ir::function::FunctionId as IrFunctionId;
use siko_ir::function::FunctionInfo;
use siko_ir::instance_resolution_cache::ResolutionResult;
use siko_ir::pattern::Pattern as IrPattern;
use siko_ir::pattern::PatternId as IrPatternId;
use siko_ir::program::Program as IrProgram;
use siko_ir::types::Type as IrType;
use siko_ir::unifier::Unifier;
use siko_location_info::item::ItemInfo;
use siko_mir::data::Adt as MirAdt;
use siko_mir::data::Record as MirRecord;
use siko_mir::data::RecordField as MirRecordField;
use siko_mir::data::TypeDef as MirTypeDef;
use siko_mir::data::TypeDefId as MirTypeDefId;
use siko_mir::data::Variant as MirVariant;
use siko_mir::expr::Expr as MirExpr;
use siko_mir::expr::ExprId as MirExprId;
use siko_mir::function::Function as MirFunction;
use siko_mir::function::FunctionId as MirFunctionId;
use siko_mir::function::FunctionInfo as MirFunctionInfo;
use siko_mir::pattern::Pattern as MirPattern;
use siko_mir::pattern::PatternId as MirPatternId;
use siko_mir::program::Program as MirProgram;
use siko_mir::types::Type as MirType;
use std::collections::BTreeMap;

pub struct TypeDefStore {
    typedefs: BTreeMap<IrType, MirTypeDefId>,
}

impl TypeDefStore {
    pub fn new() -> TypeDefStore {
        TypeDefStore {
            typedefs: BTreeMap::new(),
        }
    }

    pub fn add_type(
        &mut self,
        ty: IrType,
        ir_program: &IrProgram,
        mir_program: &mut MirProgram,
    ) -> MirTypeDefId {
        let mut newly_added = false;
        let mir_typedef_id = *self.typedefs.entry(ty.clone()).or_insert_with(|| {
            newly_added = true;
            mir_program.typedefs.get_id()
        });
        if newly_added {
            match &ty {
                IrType::Named(_, ir_typedef_id, _) => {
                    let ir_typdef = ir_program.typedefs.get(ir_typedef_id);
                    match ir_typdef {
                        IrTypeDef::Adt(_) => {
                            let mut adt_type_info = ir_program
                                .adt_type_info_map
                                .get(ir_typedef_id)
                                .expect("Adt type info not found")
                                .clone();
                            let mut unifier = ir_program.get_unifier();
                            let r = unifier.unify(&adt_type_info.adt_type, &ty);
                            assert!(r.is_ok());
                            adt_type_info.apply(&unifier);
                            let ir_adt = ir_program.typedefs.get(ir_typedef_id).get_adt();
                            let mut variants = Vec::new();
                            for (index, variant) in adt_type_info.variant_types.iter().enumerate() {
                                let mut mir_item_types = Vec::new();
                                for (item_ty, _) in &variant.item_types {
                                    let mir_item_ty =
                                        process_type(item_ty, self, ir_program, mir_program);
                                    mir_item_types.push(mir_item_ty);
                                }
                                let mir_variant = MirVariant {
                                    name: ir_adt.variants[index].name.clone(),
                                    items: mir_item_types,
                                };
                                variants.push(mir_variant);
                            }
                            let mir_adt = MirAdt {
                                id: mir_typedef_id,
                                module: ir_adt.module.clone(),
                                name: ir_adt.name.clone(),
                                variants: variants,
                            };
                            let mir_typedef = MirTypeDef::Adt(mir_adt);
                            mir_program.typedefs.add_item(mir_typedef_id, mir_typedef);
                        }
                        IrTypeDef::Record(_) => {
                            let mut record_type_info = ir_program
                                .record_type_info_map
                                .get(ir_typedef_id)
                                .expect("Record type info not found")
                                .clone();
                            let mut unifier = ir_program.get_unifier();
                            let r = unifier.unify(&record_type_info.record_type, &ty);
                            assert!(r.is_ok());
                            record_type_info.apply(&unifier);
                            let ir_record = ir_program.typedefs.get(ir_typedef_id).get_record();
                            let mut fields = Vec::new();
                            for (index, (field_ty, _)) in
                                record_type_info.field_types.iter().enumerate()
                            {
                                let mir_field_ty =
                                    process_type(field_ty, self, ir_program, mir_program);
                                let mir_field = MirRecordField {
                                    name: ir_record.fields[index].name.clone(),
                                    ty: mir_field_ty,
                                };
                                fields.push(mir_field);
                            }
                            let mir_record = MirRecord {
                                id: mir_typedef_id,
                                module: ir_record.module.clone(),
                                name: ir_record.name.clone(),
                                fields: fields,
                                external: ir_record.external,
                            };
                            let mir_typedef = MirTypeDef::Record(mir_record);
                            mir_program.typedefs.add_item(mir_typedef_id, mir_typedef);
                        }
                    }
                }
                _ => unreachable!(),
            };
        }
        mir_typedef_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FunctionQueueItem {
    function_id: IrFunctionId,
    unifier: Unifier,
}

impl FunctionQueueItem {
    pub fn new(function_id: IrFunctionId, unifier: Unifier) -> FunctionQueueItem {
        FunctionQueueItem {
            function_id: function_id,
            unifier: unifier,
        }
    }
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
        ir_program: &IrProgram,
        mir_program: &mut MirProgram,
        typedef_store: &mut TypeDefStore,
    ) {
        while !self.pending.is_empty() {
            if let Some((item, mir_function_id)) = self.pending.pop() {
                process_function(
                    &item.function_id,
                    mir_function_id,
                    ir_program,
                    mir_program,
                    &item.unifier,
                    self,
                    typedef_store,
                );
            }
        }
    }
}

fn process_type(
    ir_type: &IrType,
    typedef_store: &mut TypeDefStore,
    ir_program: &IrProgram,
    mir_program: &mut MirProgram,
) -> MirType {
    match ir_type {
        IrType::FixedTypeArg(..) => unreachable!(),
        IrType::Var(..) => unreachable!(),
        IrType::Function(from, to) => {
            let from = process_type(from, typedef_store, ir_program, mir_program);
            let to = process_type(to, typedef_store, ir_program, mir_program);
            MirType::Function(Box::new(from), Box::new(to))
        }
        IrType::Named(name, _, _) => {
            let mir_typedef_id = typedef_store.add_type(ir_type.clone(), ir_program, mir_program);
            MirType::Named(format!("{}/{}", name, mir_typedef_id.id), mir_typedef_id)
        }
        IrType::Tuple(items) => {
            let items: Vec<_> = items
                .iter()
                .map(|item| process_type(item, typedef_store, ir_program, mir_program))
                .collect();
            MirType::Tuple(items)
        }
    }
}

fn get_call_unifier(
    arg_types: &Vec<IrType>,
    func_ty: &IrType,
    expected_result_ty: &IrType,
    ir_program: &IrProgram,
) -> Unifier {
    for arg in arg_types {
        assert!(arg.is_concrete_type());
    }
    let mut call_unifier = ir_program.get_unifier();
    let mut func_ty = func_ty.clone();
    for arg in arg_types {
        let mut func_arg_types = Vec::new();
        func_ty.get_args(&mut func_arg_types);
        let r = call_unifier.unify(&arg, &func_arg_types[0]);
        assert!(r.is_ok());
        func_ty.apply(&call_unifier);
        func_ty = func_ty.get_result_type(1);
    }
    //println!("{} {}", expected_result_ty, func_ty);
    let r = call_unifier.unify(&func_ty, expected_result_ty);
    assert!(r.is_ok());
    call_unifier
}

fn process_pattern(
    ir_pattern_id: &IrPatternId,
    ir_program: &IrProgram,
    mir_program: &mut MirProgram,
    unifier: &Unifier,
    function_queue: &mut FunctionQueue,
    typedef_store: &mut TypeDefStore,
) -> MirPatternId {
    let item_info = &ir_program.patterns.get(ir_pattern_id);
    let mut ir_pattern_ty = ir_program.get_pattern_type(ir_pattern_id).clone();
    ir_pattern_ty.apply(unifier);
    let mir_pattern_ty = process_type(&ir_pattern_ty, typedef_store, ir_program, mir_program);
    let pattern = &item_info.item;
    let mir_pattern = match pattern {
        IrPattern::Binding(name) => MirPattern::Binding(name.clone()),
        IrPattern::FloatLiteral(v) => MirPattern::FloatLiteral(v.clone()),
        IrPattern::Guarded(sub, guard_expr) => {
            let mir_sub = process_pattern(
                sub,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            let mir_guard_expr = process_expr(
                guard_expr,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            MirPattern::Guarded(mir_sub, mir_guard_expr)
        }
        IrPattern::IntegerLiteral(v) => MirPattern::IntegerLiteral(v.clone()),
        IrPattern::Record(_, items) => {
            let mir_typedef_id = typedef_store.add_type(ir_pattern_ty, ir_program, mir_program);
            let mir_items: Vec<_> = items
                .iter()
                .map(|item| {
                    process_pattern(
                        item,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    )
                })
                .collect();
            MirPattern::Record(mir_typedef_id, mir_items)
        }
        IrPattern::StringLiteral(v) => MirPattern::StringLiteral(v.clone()),
        IrPattern::Tuple(items) => {
            let mir_items: Vec<_> = items
                .iter()
                .map(|item| {
                    process_pattern(
                        item,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    )
                })
                .collect();
            MirPattern::Tuple(mir_items)
        }
        IrPattern::Typed(sub, _) => {
            return process_pattern(
                sub,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
        }
        IrPattern::Variant(_, index, items) => {
            let mir_typedef_id = typedef_store.add_type(ir_pattern_ty, ir_program, mir_program);
            let mir_items: Vec<_> = items
                .iter()
                .map(|item| {
                    process_pattern(
                        item,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    )
                })
                .collect();
            MirPattern::Variant(mir_typedef_id, *index, mir_items)
        }
        IrPattern::Wildcard => MirPattern::Wildcard,
    };
    return mir_program.add_pattern(mir_pattern, item_info.location_id, mir_pattern_ty);
}

fn process_class_member_call(
    arg_types: &Vec<IrType>,
    ir_program: &IrProgram,
    class_member_id: &ClassMemberId,
    expr_ty: IrType,
) -> (IrFunctionId, Unifier) {
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
    let class = ir_program.classes.get(&member.class_id);
    assert!(class_arg.is_concrete_type());
    match cache.get(member.class_id, class_arg) {
        ResolutionResult::AutoDerived => {
            panic!(
                "Auto derive of {}/{} is not implemented",
                class.module, class.name
            );
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
            (member_function_id, call_unifier)
        }
    }
}

fn process_expr(
    ir_expr_id: &IrExprId,
    ir_program: &IrProgram,
    mir_program: &mut MirProgram,
    unifier: &Unifier,
    function_queue: &mut FunctionQueue,
    typedef_store: &mut TypeDefStore,
) -> MirExprId {
    let item_info = &ir_program.exprs.get(ir_expr_id);
    let expr = &item_info.item;
    let mut ir_expr_ty = ir_program.get_expr_type(&ir_expr_id).clone();
    ir_expr_ty.apply(unifier);
    let mir_expr_ty = process_type(&ir_expr_ty, typedef_store, ir_program, mir_program);
    let mir_expr = match expr {
        IrExpr::ArgRef(arg_ref) => {
            assert!(!arg_ref.captured);
            MirExpr::ArgRef(arg_ref.index)
        }
        IrExpr::Bind(pattern_id, rhs) => {
            let mir_pattern_id = process_pattern(
                pattern_id,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            let mir_rhs = process_expr(
                rhs,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            MirExpr::Bind(mir_pattern_id, mir_rhs)
        }
        IrExpr::ClassFunctionCall(class_member_id, args) => {
            let mir_args: Vec<_> = args
                .iter()
                .map(|arg| {
                    process_expr(
                        arg,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    )
                })
                .collect();
            let mut arg_types: Vec<_> = args
                .iter()
                .map(|arg| ir_program.get_expr_type(arg).clone())
                .collect();
            for arg_type in &mut arg_types {
                arg_type.apply(unifier);
            }
            let (func_id, call_unifier) =
                process_class_member_call(&arg_types, ir_program, class_member_id, ir_expr_ty);
            let queue_item = FunctionQueueItem::new(func_id, call_unifier);
            let mir_function_id = function_queue.insert(queue_item, mir_program);
            MirExpr::StaticFunctionCall(mir_function_id, mir_args)
        }
        IrExpr::Do(items) => {
            let mir_items: Vec<_> = items
                .iter()
                .map(|item| {
                    process_expr(
                        item,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    )
                })
                .collect();
            MirExpr::Do(mir_items)
        }
        IrExpr::DynamicFunctionCall(func_expr_id, args) => {
            let mir_func_expr_id = process_expr(
                func_expr_id,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            let mir_args: Vec<_> = args
                .iter()
                .map(|arg| {
                    process_expr(
                        arg,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    )
                })
                .collect();
            MirExpr::DynamicFunctionCall(mir_func_expr_id, mir_args)
        }
        IrExpr::ExprValue(expr_id, pattern_id) => {
            let mir_expr_id = process_expr(
                expr_id,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            let mir_pattern_id = process_pattern(
                pattern_id,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            MirExpr::ExprValue(mir_expr_id, mir_pattern_id)
        }
        IrExpr::Formatter(fmt, args) => {
            let mir_args: Vec<_> = args
                .iter()
                .map(|arg| {
                    process_expr(
                        arg,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    )
                })
                .collect();
            MirExpr::Formatter(fmt.clone(), mir_args)
        }
        IrExpr::IntegerLiteral(value) => MirExpr::IntegerLiteral(*value),
        IrExpr::List(items) => {
            let mir_items: Vec<_> = items
                .iter()
                .map(|item| {
                    process_expr(
                        item,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    )
                })
                .collect();
            MirExpr::List(mir_items)
        }
        IrExpr::StaticFunctionCall(func_id, args) => {
            let function_type = ir_program.get_function_type(func_id).remove_fixed_types();
            let mut arg_types: Vec<_> = args
                .iter()
                .map(|arg| ir_program.get_expr_type(arg).clone())
                .collect();
            for arg_type in &mut arg_types {
                arg_type.apply(unifier);
            }
            let call_unifier =
                get_call_unifier(&arg_types, &function_type, &ir_expr_ty, ir_program);
            let mir_args: Vec<_> = args
                .iter()
                .map(|arg| {
                    process_expr(
                        arg,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    )
                })
                .collect();
            let queue_item = FunctionQueueItem::new(*func_id, call_unifier);
            let mir_function_id = function_queue.insert(queue_item, mir_program);
            MirExpr::StaticFunctionCall(mir_function_id, mir_args)
        }
        IrExpr::StringLiteral(value) => MirExpr::StringLiteral(value.clone()),
        IrExpr::Tuple(items) => {
            let mir_items: Vec<_> = items
                .iter()
                .map(|item| {
                    process_expr(
                        item,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    )
                })
                .collect();
            MirExpr::Tuple(mir_items)
        }
        _ => {
            panic!("Compiling expr {} is not yet implemented", expr);
        }
    };
    return mir_program.add_expr(mir_expr, item_info.location_id, mir_expr_ty);
}

fn process_function(
    ir_function_id: &IrFunctionId,
    mir_function_id: MirFunctionId,
    ir_program: &IrProgram,
    mir_program: &mut MirProgram,
    unifier: &Unifier,
    function_queue: &mut FunctionQueue,
    typedef_store: &mut TypeDefStore,
) {
    let mut function_type = ir_program.get_function_type(ir_function_id).clone();
    function_type.apply(unifier);
    let mir_function_type = process_type(&function_type, typedef_store, ir_program, mir_program);
    let function = ir_program.functions.get(ir_function_id);
    match &function.info {
        FunctionInfo::NamedFunction(info) => {
            let mir_function_info = if let Some(body) = info.body {
                let mir_expr_id = process_expr(
                    &body,
                    ir_program,
                    mir_program,
                    unifier,
                    function_queue,
                    typedef_store,
                );
                MirFunctionInfo::Normal(mir_expr_id)
            } else {
                MirFunctionInfo::Extern
            };
            let mir_function = MirFunction {
                name: format!("{}", info),
                info: mir_function_info,
                function_type: mir_function_type,
            };
            mir_program
                .functions
                .add_item(mir_function_id, mir_function);
        }
        FunctionInfo::Lambda(info) => {
            let mir_body = process_expr(
                &info.body,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            let mir_function = MirFunction {
                name: format!("{}", info),
                info: MirFunctionInfo::Normal(mir_body),
                function_type: mir_function_type,
            };
            mir_program
                .functions
                .add_item(mir_function_id, mir_function);
        }
        FunctionInfo::VariantConstructor(info) => {
            let result_ty = function_type.get_result_type(function.arg_count);
            let mir_typedef_id = typedef_store.add_type(result_ty, ir_program, mir_program);
            let mir_function = MirFunction {
                name: format!("{}", info),
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

pub struct Backend {}

impl Backend {
    pub fn compile(ir_program: &IrProgram) -> Result<MirProgram, ()> {
        let unifier = ir_program.get_unifier();
        let mut mir_program = MirProgram::new();
        let mut function_queue = FunctionQueue::new();
        let mut typedef_store = TypeDefStore::new();
        let main_id = ir_program.get_main().expect("Main not found");
        function_queue.insert(FunctionQueueItem::new(main_id, unifier), &mut mir_program);
        function_queue.process_items(ir_program, &mut mir_program, &mut typedef_store);
        for (id, function) in mir_program.functions.items.iter() {
            println!(
                "{}, {} {}",
                id,
                function.name,
                function.function_type.as_string()
            );
        }
        Ok(mir_program)
    }
}
