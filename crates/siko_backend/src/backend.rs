use siko_ir::class::ClassMemberId;
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
use siko_mir::expr::Expr as MirExpr;
use siko_mir::expr::ExprId as MirExprId;
use siko_mir::function::Function as MirFunction;
use siko_mir::function::FunctionId as MirFunctionId;
use siko_mir::pattern::Pattern as MirPattern;
use siko_mir::pattern::PatternId as MirPatternId;
use siko_mir::program::Program as MirProgram;
use siko_mir::types::Type as MirType;
use std::collections::BTreeMap;

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

    pub fn process_items(&mut self, ir_program: &IrProgram, mir_program: &mut MirProgram) {
        while !self.pending.is_empty() {
            if let Some((item, mir_function_id)) = self.pending.pop() {
                process_function(
                    &item.function_id,
                    mir_function_id,
                    ir_program,
                    mir_program,
                    &item.unifier,
                    self,
                );
            }
        }
    }
}

fn process_type(ir_type: &IrType) -> MirType {
    match ir_type {
        IrType::FixedTypeArg(..) => unreachable!(),
        IrType::Var(..) => unreachable!(),
        IrType::Function(from, to) => {
            let from = process_type(from);
            let to = process_type(to);
            MirType::Function(Box::new(from), Box::new(to))
        }
        IrType::Named(name, id, args) => {
            let args: Vec<_> = args.iter().map(|arg| process_type(arg)).collect();
            MirType::Named(name.clone(), 0.into(), args)
        }
        IrType::Tuple(items) => {
            let items: Vec<_> = items.iter().map(|item| process_type(item)).collect();
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
) -> MirPatternId {
    let item_info = &ir_program.patterns.get(ir_pattern_id);
    let pattern = &item_info.item;
    let mir_pattern = match pattern {
        IrPattern::Binding(name) => MirPattern::Binding(name.clone()),
        IrPattern::Tuple(items) => {
            let mir_items: Vec<_> = items
                .iter()
                .map(|item| process_pattern(item, ir_program, mir_program, unifier, function_queue))
                .collect();
            MirPattern::Tuple(mir_items)
        }
        _ => {
            panic!("Compiling pattern {:?} is not yet implemented", pattern);
        }
    };
    let mir_pattern_info = ItemInfo {
        item: mir_pattern,
        location_id: item_info.location_id,
    };
    let mir_pattern_id = mir_program.patterns.get_id();
    mir_program
        .patterns
        .add_item(mir_pattern_id, mir_pattern_info);
    mir_pattern_id
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
) -> MirExprId {
    let item_info = &ir_program.exprs.get(ir_expr_id);
    let expr = &item_info.item;
    let mir_expr = match expr {
        IrExpr::ArgRef(arg_ref) => {
            assert!(!arg_ref.captured);
            MirExpr::ArgRef(arg_ref.index)
        }
        IrExpr::Bind(pattern_id, rhs) => {
            let mir_pattern_id =
                process_pattern(pattern_id, ir_program, mir_program, unifier, function_queue);
            let mir_rhs = process_expr(rhs, ir_program, mir_program, unifier, function_queue);
            MirExpr::Bind(mir_pattern_id, mir_rhs)
        }
        IrExpr::ClassFunctionCall(class_member_id, args) => {
            let mir_args: Vec<_> = args
                .iter()
                .map(|arg| process_expr(arg, ir_program, mir_program, unifier, function_queue))
                .collect();
            let mut arg_types: Vec<_> = args
                .iter()
                .map(|arg| ir_program.get_expr_type(arg).clone())
                .collect();
            for arg_type in &mut arg_types {
                arg_type.apply(unifier);
            }
            let mut expr_ty = ir_program.get_expr_type(&ir_expr_id).clone();
            expr_ty.apply(unifier);
            let (func_id, call_unifier) =
                process_class_member_call(&arg_types, ir_program, class_member_id, expr_ty);
            let queue_item = FunctionQueueItem::new(func_id, call_unifier);
            let mir_function_id = function_queue.insert(queue_item, mir_program);
            MirExpr::StaticFunctionCall(mir_function_id, mir_args)
        }
        IrExpr::Do(items) => {
            let mir_items: Vec<_> = items
                .iter()
                .map(|item| process_expr(item, ir_program, mir_program, unifier, function_queue))
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
            );
            let mir_args: Vec<_> = args
                .iter()
                .map(|arg| process_expr(arg, ir_program, mir_program, unifier, function_queue))
                .collect();
            MirExpr::DynamicFunctionCall(mir_func_expr_id, mir_args)
        }
        IrExpr::ExprValue(expr_id, pattern_id) => {
            let mir_expr_id =
                process_expr(expr_id, ir_program, mir_program, unifier, function_queue);
            let mir_pattern_id =
                process_pattern(pattern_id, ir_program, mir_program, unifier, function_queue);
            MirExpr::ExprValue(mir_expr_id, mir_pattern_id)
        }
        IrExpr::Formatter(fmt, args) => {
            let mir_args: Vec<_> = args
                .iter()
                .map(|arg| process_expr(arg, ir_program, mir_program, unifier, function_queue))
                .collect();
            MirExpr::Formatter(fmt.clone(), mir_args)
        }
        IrExpr::IntegerLiteral(value) => MirExpr::IntegerLiteral(*value),
        IrExpr::List(items) => {
            let mir_items: Vec<_> = items
                .iter()
                .map(|item| process_expr(item, ir_program, mir_program, unifier, function_queue))
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
            let mut expected_result_ty = ir_program.get_expr_type(&ir_expr_id).clone();
            expected_result_ty.apply(unifier);
            let call_unifier =
                get_call_unifier(&arg_types, &function_type, &expected_result_ty, ir_program);
            let mir_args: Vec<_> = args
                .iter()
                .map(|arg| process_expr(arg, ir_program, mir_program, unifier, function_queue))
                .collect();
            let queue_item = FunctionQueueItem::new(*func_id, call_unifier);
            let mir_function_id = function_queue.insert(queue_item, mir_program);
            MirExpr::StaticFunctionCall(mir_function_id, mir_args)
        }
        IrExpr::StringLiteral(value) => MirExpr::StringLiteral(value.clone()),
        IrExpr::Tuple(items) => {
            let mir_items: Vec<_> = items
                .iter()
                .map(|item| process_expr(item, ir_program, mir_program, unifier, function_queue))
                .collect();
            MirExpr::Tuple(mir_items)
        }
        _ => {
            panic!("Compiling expr {} is not yet implemented", expr);
        }
    };
    let mir_expr_info = ItemInfo {
        item: mir_expr,
        location_id: item_info.location_id,
    };
    let mir_expr_id = mir_program.exprs.get_id();
    mir_program.exprs.add_item(mir_expr_id, mir_expr_info);
    mir_expr_id
}

fn process_function(
    ir_function_id: &IrFunctionId,
    mir_function_id: MirFunctionId,
    ir_program: &IrProgram,
    mir_program: &mut MirProgram,
    unifier: &Unifier,
    function_queue: &mut FunctionQueue,
) {
    let mut function_type = ir_program.get_function_type(ir_function_id).clone();
    function_type.apply(unifier);
    let mir_function_type = process_type(&function_type);
    let function = ir_program.functions.get(ir_function_id);
    match &function.info {
        FunctionInfo::NamedFunction(info) => {
            let mir_body = if let Some(body) = info.body {
                let mir_expr_id =
                    process_expr(&body, ir_program, mir_program, unifier, function_queue);
                Some(mir_expr_id)
            } else {
                None
            };
            let mir_function = MirFunction {
                name: format!("{}", info),
                body: mir_body,
                function_type: mir_function_type,
            };
            mir_program
                .functions
                .add_item(mir_function_id, mir_function);
        }
        FunctionInfo::Lambda(info) => {
            let mir_body =
                process_expr(&info.body, ir_program, mir_program, unifier, function_queue);
            let mir_function = MirFunction {
                name: format!("{}", info),
                body: Some(mir_body),
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
        let main_id = ir_program.get_main().expect("Main not found");
        function_queue.insert(FunctionQueueItem::new(main_id, unifier), &mut mir_program);
        function_queue.process_items(ir_program, &mut mir_program);
        for (id, function) in mir_program.functions.items.iter() {
            println!("{}, {} {:?}", id, function.name, function.function_type);
        }
        Ok(mir_program)
    }
}
