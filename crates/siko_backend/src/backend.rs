use crate::format_rewriter::FormatRewriter;
use siko_constants::BOOL_MODULE_NAME;
use siko_constants::BOOL_TYPE_NAME;
use siko_constants::FALSE_NAME;
use siko_constants::TRUE_NAME;
use siko_ir::class::ClassId;
use siko_ir::class::ClassMemberId;
use siko_ir::data::Record as IrRecord;
use siko_ir::data::TypeDef as IrTypeDef;
use siko_ir::data_type_info::RecordTypeInfo;
use siko_ir::expr::Expr as IrExpr;
use siko_ir::expr::ExprId as IrExprId;
use siko_ir::expr::FunctionArgumentRef;
use siko_ir::function::Function as IrFunction;
use siko_ir::function::FunctionId as IrFunctionId;
use siko_ir::function::FunctionInfo;
use siko_ir::function::NamedFunctionInfo;
use siko_ir::function::NamedFunctionKind;
use siko_ir::instance_resolution_cache::ResolutionResult;
use siko_ir::pattern::Pattern as IrPattern;
use siko_ir::pattern::PatternId as IrPatternId;
use siko_ir::program::Program as IrProgram;
use siko_ir::types::Type as IrType;
use siko_ir::unifier::Unifier;
use siko_ir::walker::walk_expr;
use siko_location_info::item::ItemInfo;
use siko_location_info::location_id::LocationId;
use siko_mir::data::Adt as MirAdt;
use siko_mir::data::Record as MirRecord;
use siko_mir::data::RecordField as MirRecordField;
use siko_mir::data::TypeDef as MirTypeDef;
use siko_mir::data::TypeDefId as MirTypeDefId;
use siko_mir::data::Variant as MirVariant;
use siko_mir::expr::Case as MirCase;
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

    pub fn add_tuple(
        &mut self,
        ty: IrType,
        field_types: Vec<MirType>,
        mir_program: &mut MirProgram,
    ) -> (String, MirTypeDefId) {
        let mut newly_added = false;
        let mir_typedef_id = *self.typedefs.entry(ty.clone()).or_insert_with(|| {
            newly_added = true;
            mir_program.typedefs.get_id()
        });
        let name = format!("tuple#{}", mir_typedef_id.id);
        if newly_added {
            let mut fields = Vec::new();
            for (index, field_ty) in field_types.into_iter().enumerate() {
                let mir_field = MirRecordField {
                    name: format!("field#{}", index),
                    ty: field_ty,
                };
                fields.push(mir_field);
            }
            let mir_record = MirRecord {
                id: mir_typedef_id,
                module: format!("<generated>"),
                name: name.clone(),
                fields: fields,
                external: false,
            };
            let mir_typedef = MirTypeDef::Record(mir_record);
            mir_program.typedefs.add_item(mir_typedef_id, mir_typedef);
        }
        (name, mir_typedef_id)
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

struct FunctionBuilder<'a> {
    ir_program: &'a mut IrProgram,
}

impl<'a> FunctionBuilder<'a> {
    pub fn new(ir_program: &'a mut IrProgram) -> FunctionBuilder<'a> {
        FunctionBuilder {
            ir_program: ir_program,
        }
    }

    pub fn create_bool(&mut self, value: bool, location: LocationId) -> IrExprId {
        let bool_ty = self.ir_program.get_bool_type();
        let ctor = self.ir_program.get_constructor_by_name(
            BOOL_MODULE_NAME,
            BOOL_TYPE_NAME,
            if value { TRUE_NAME } else { FALSE_NAME },
        );
        let expr = IrExpr::StaticFunctionCall(ctor, vec![]);
        self.add_expr(expr, location, bool_ty)
    }

    pub fn add_arg_ref(
        &mut self,
        index: usize,
        function_id: IrFunctionId,
        location: LocationId,
        arg_ty: IrType,
    ) -> IrExprId {
        let arg_ref = FunctionArgumentRef::new(false, function_id, index);
        let arg_ref_expr = IrExpr::ArgRef(arg_ref);
        let arg_ref_expr_id = self.add_expr(arg_ref_expr, location, arg_ty);
        arg_ref_expr_id
    }

    pub fn add_record_pattern(
        &mut self,
        source_expr: IrExprId,
        record: &IrRecord,
        record_type_info: &RecordTypeInfo,
        location: LocationId,
    ) -> (IrExprId, Vec<IrExprId>) {
        let mut field_patterns = Vec::new();
        let mut values = Vec::new();
        for (index, (field_type, _)) in record_type_info.field_types.iter().enumerate() {
            let field = &record.fields[index];
            let field_pattern = IrPattern::Binding(field.name.clone());
            let field_pattern_id = self.add_pattern(field_pattern, location, field_type.clone());
            field_patterns.push(field_pattern_id);
            let expr_value_expr = IrExpr::ExprValue(source_expr, field_pattern_id);
            let expr_value_expr_id = self.add_expr(expr_value_expr, location, field_type.clone());
            values.push(expr_value_expr_id);
        }
        let pattern = IrPattern::Record(record.id, field_patterns);
        let pattern_id = self.add_pattern(pattern, location, record_type_info.record_type.clone());
        let bind_expr = IrExpr::Bind(pattern_id, source_expr);
        let bind_expr_id = self.add_expr(bind_expr, location, IrType::Tuple(vec![]));
        (bind_expr_id, values)
    }

    pub fn add_expr(&mut self, expr: IrExpr, location_id: LocationId, expr_ty: IrType) -> IrExprId {
        let id = self.ir_program.exprs.get_id();
        self.ir_program
            .exprs
            .add_item(id, ItemInfo::new(expr, location_id));
        self.ir_program.expr_types.insert(id, expr_ty);
        id
    }

    pub fn add_pattern(
        &mut self,
        pattern: IrPattern,
        location_id: LocationId,
        pattern_ty: IrType,
    ) -> IrPatternId {
        let id = self.ir_program.patterns.get_id();
        self.ir_program
            .patterns
            .add_item(id, ItemInfo::new(pattern, location_id));
        self.ir_program.pattern_types.insert(id, pattern_ty);
        id
    }
}

#[derive(Debug)]
enum DerivedClass {
    Show,
    PartialEq,
    PartialOrd,
    Ord,
}

fn generate_show_instance_member_for_record(
    ir_program: &mut IrProgram,
    location: LocationId,
    function_id: IrFunctionId,
    record: &IrRecord,
    record_type_info: RecordTypeInfo,
) -> (IrExprId, IrType) {
    let string_ty = ir_program.get_string_type();
    let mut builder = FunctionBuilder::new(ir_program);
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

fn generate_partialeq_instance_member_for_record(
    ir_program: &mut IrProgram,
    location: LocationId,
    function_id: IrFunctionId,
    record: &IrRecord,
    record_type_info: RecordTypeInfo,
    class_member_id: ClassMemberId,
) -> (IrExprId, IrType) {
    let bool_ty = ir_program.get_bool_type();
    let mut builder = FunctionBuilder::new(ir_program);
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

fn generate_auto_derived_instance_member(
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FunctionQueueItem {
    Normal(IrFunctionId, Unifier),
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
                    FunctionQueueItem::Normal(function_id, unifier) => {
                        process_function(
                            &function_id,
                            mir_function_id,
                            ir_program,
                            mir_program,
                            &unifier,
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
                                    mir_program,
                                    self,
                                    DerivedClass::Show,
                                    class_member_id,
                                );
                            }
                            ("Std.Ops", "PartialEq") => {
                                generate_auto_derived_instance_member(
                                    class_id,
                                    &ir_type,
                                    ir_program,
                                    mir_program,
                                    self,
                                    DerivedClass::PartialEq,
                                    class_member_id,
                                );
                            }
                            ("Std.Ops", "PartialOrd") => {
                                generate_auto_derived_instance_member(
                                    class_id,
                                    &ir_type,
                                    ir_program,
                                    mir_program,
                                    self,
                                    DerivedClass::PartialOrd,
                                    class_member_id,
                                );
                            }
                            ("Std.Ops", "Ord") => {
                                generate_auto_derived_instance_member(
                                    class_id,
                                    &ir_type,
                                    ir_program,
                                    mir_program,
                                    self,
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
            let (name, mir_typedef_id) =
                typedef_store.add_tuple(ir_type.clone(), items, mir_program);
            MirType::Named(name, mir_typedef_id)
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
        IrExpr::CaseOf(body, cases, _) => {
            let mir_body = process_expr(
                body,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            let cases: Vec<_> = cases
                .iter()
                .map(|case| {
                    let mir_case_body = process_expr(
                        &case.body,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    );
                    let mir_case_pattern = process_pattern(
                        &case.pattern_id,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    );
                    MirCase {
                        body: mir_case_body,
                        pattern_id: mir_case_pattern,
                    }
                })
                .collect();
            MirExpr::CaseOf(mir_body, cases)
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
            let mir_function_id = process_class_member_call(
                &arg_types,
                ir_program,
                mir_program,
                class_member_id,
                ir_expr_ty,
                function_queue,
            );
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
        IrExpr::FieldAccess(infos, receiver_expr_id) => {
            assert_eq!(infos.len(), 1);
            let mir_receiver_expr_id = process_expr(
                receiver_expr_id,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            MirExpr::FieldAccess(infos[0].index, mir_receiver_expr_id)
        }
        IrExpr::FloatLiteral(v) => MirExpr::FloatLiteral(*v),
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
        IrExpr::If(cond, true_branch, false_branch) => {
            let mir_cond = process_expr(
                cond,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            let mir_true_branch = process_expr(
                true_branch,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            let mir_false_branch = process_expr(
                false_branch,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            MirExpr::If(mir_cond, mir_true_branch, mir_false_branch)
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
        IrExpr::RecordInitialization(_, fields) => {
            let mir_fields = fields
                .iter()
                .map(|field| {
                    let field_expr = process_expr(
                        &field.expr_id,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    );
                    (field_expr, field.index)
                })
                .collect();
            MirExpr::RecordInitialization(mir_fields)
        }
        IrExpr::RecordUpdate(receiver_expr_id, updates) => {
            let mir_receiver_expr_id = process_expr(
                receiver_expr_id,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            assert_eq!(updates.len(), 1);
            let mir_updates = updates[0]
                .items
                .iter()
                .map(|item| {
                    let field_expr = process_expr(
                        &item.expr_id,
                        ir_program,
                        mir_program,
                        unifier,
                        function_queue,
                        typedef_store,
                    );
                    (field_expr, item.index)
                })
                .collect();
            MirExpr::RecordUpdate(mir_receiver_expr_id, mir_updates)
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
            let queue_item = FunctionQueueItem::Normal(*func_id, call_unifier);
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
        IrExpr::TupleFieldAccess(index, receiver_expr_id) => {
            let mir_receiver_expr_id = process_expr(
                receiver_expr_id,
                ir_program,
                mir_program,
                unifier,
                function_queue,
                typedef_store,
            );
            MirExpr::FieldAccess(*index, mir_receiver_expr_id)
        }
    };
    return mir_program.add_expr(mir_expr, item_info.location_id, mir_expr_ty);
}

fn process_function(
    ir_function_id: &IrFunctionId,
    mir_function_id: MirFunctionId,
    ir_program: &mut IrProgram,
    mir_program: &mut MirProgram,
    unifier: &Unifier,
    function_queue: &mut FunctionQueue,
    typedef_store: &mut TypeDefStore,
) {
    let mut function_type = ir_program.get_function_type(ir_function_id).clone();
    function_type.apply(unifier);
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
                    unifier,
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
                unifier,
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
            let module = adt.module.clone();
            let result_ty = function_type.get_result_type(function.arg_count);
            let mir_typedef_id = typedef_store.add_type(result_ty, ir_program, mir_program);
            let mir_function = MirFunction {
                name: format!("{}_ctor", adt.name),
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

fn preprocess_ir(body: IrExprId, ir_program: &mut IrProgram) {
    let mut rewriter = FormatRewriter::new(ir_program);
    walk_expr(&body, &mut rewriter);
}

pub struct Backend {}

impl Backend {
    pub fn compile(ir_program: &mut IrProgram) -> Result<MirProgram, ()> {
        let unifier = ir_program.get_unifier();
        let mut mir_program = MirProgram::new();
        let mut function_queue = FunctionQueue::new();
        let mut typedef_store = TypeDefStore::new();
        let main_id = ir_program.get_main().expect("Main not found");
        function_queue.insert(
            FunctionQueueItem::Normal(main_id, unifier),
            &mut mir_program,
        );
        function_queue.process_items(ir_program, &mut mir_program, &mut typedef_store);
        Ok(mir_program)
    }
}
