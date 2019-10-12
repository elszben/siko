use crate::common::create_general_function_type;
use crate::common::ClassMemberTypeInfo;
use crate::common::FunctionTypeInfo;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::error::TypecheckError;
use crate::type_processor::process_type_signature;
use crate::type_store::TypeStore;
use crate::types::Type;
use siko_ir::class::ClassMemberId;
use siko_ir::expr::ExprId;
use siko_ir::function::Function;
use siko_ir::function::FunctionId;
use siko_ir::function::FunctionInfo;
use siko_ir::function::NamedFunctionKind;
use siko_ir::program::Program;
use siko_ir::types::TypeDefId;
use siko_ir::types::TypeSignatureId;
use siko_location_info::item::LocationId;
use std::collections::BTreeMap;

pub struct FunctionProcessor<'a> {
    type_store: &'a mut TypeStore,
    function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    record_type_info_map: &'a BTreeMap<TypeDefId, RecordTypeInfo>,
    variant_type_info_map: &'a BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
}

impl<'a> FunctionProcessor<'a> {
    pub fn new(
        type_store: &'a mut TypeStore,
        record_type_info_map: &'a BTreeMap<TypeDefId, RecordTypeInfo>,
        variant_type_info_map: &'a BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    ) -> FunctionProcessor<'a> {
        FunctionProcessor {
            type_store: type_store,
            function_type_info_map: BTreeMap::new(),
            record_type_info_map: record_type_info_map,
            variant_type_info_map: variant_type_info_map,
        }
    }

    fn register_typed_function(
        &mut self,
        displayed_name: String,
        name: &String,
        arg_count: usize,
        type_signature_id: TypeSignatureId,
        function_id: FunctionId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
        body: Option<ExprId>,
        location_id: LocationId,
        kind: &NamedFunctionKind,
        class_member_type_info_map: &BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
    ) {
        let (arg_map, func_type_var) =
            if let NamedFunctionKind::DefaultClassMember(class_member_id) = kind {
                let info = class_member_type_info_map
                    .get(class_member_id)
                    .expect("class member type info not found");
                (info.arg_map.clone(), info.member_type_var)
            } else {
                let mut arg_map = BTreeMap::new();
                let func_type_var = process_type_signature(
                    &mut self.type_store,
                    &type_signature_id,
                    program,
                    &mut arg_map,
                    &mut None,
                );
                (arg_map, func_type_var)
            };
        let is_member = *kind != NamedFunctionKind::Free;

        /*
        println!(
            "Registering named function {} {} with type {}",
            function_id,
            displayed_name,
            self.type_store.get_resolved_type_string(&func_type_var, program)
        );
        */
        let ty = self.type_store.get_type(&func_type_var);
        let function_type_info = match ty {
            Type::Function(func_type) => {
                let mut signature_vars = Vec::new();
                func_type.get_arg_types(&self.type_store, &mut signature_vars);
                if signature_vars.len() < arg_count {
                    let location = if is_member {
                        location_id
                    } else {
                        program.type_signatures.get(&type_signature_id).location_id
                    };
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        name.clone(),
                        arg_count,
                        signature_vars.len(),
                        location,
                        is_member,
                    );
                    errors.push(err);
                    return;
                }
                let arg_vars: Vec<_> = signature_vars.iter().take(arg_count).cloned().collect();

                let return_value_var = func_type.get_return_type(&self.type_store, arg_vars.len());
                FunctionTypeInfo::new(
                    displayed_name,
                    arg_vars,
                    true,
                    return_value_var,
                    func_type_var,
                    body,
                    location_id,
                    arg_map,
                )
            }
            _ => {
                if arg_count > 0 {
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        displayed_name,
                        arg_count,
                        0,
                        program.type_signatures.get(&type_signature_id).location_id,
                        is_member,
                    );
                    errors.push(err);
                    return;
                }
                FunctionTypeInfo::new(
                    name.clone(),
                    vec![],
                    true,
                    func_type_var,
                    func_type_var,
                    body,
                    location_id,
                    arg_map,
                )
            }
        };
        self.function_type_info_map
            .insert(function_id, function_type_info);
    }

    fn register_untyped_function(
        &mut self,
        name: String,
        id: FunctionId,
        function: &Function,
        body: ExprId,
        location_id: LocationId,
    ) {
        let mut args = Vec::new();

        let (func_type_var, result) = create_general_function_type(
            function.arg_locations.len() + function.implicit_arg_count,
            &mut args,
            &mut self.type_store,
        );
        let function_type_info = FunctionTypeInfo::new(
            name,
            args,
            false,
            result,
            func_type_var,
            Some(body),
            location_id,
            BTreeMap::new(),
        );
        self.function_type_info_map.insert(id, function_type_info);
    }

    pub fn process_functions(
        mut self,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
        class_member_type_info_map: &BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
    ) -> BTreeMap<FunctionId, FunctionTypeInfo> {
        for (id, function) in &program.functions.items {
            let displayed_name = format!("{}", function.info);
            match &function.info {
                FunctionInfo::RecordConstructor(i) => {
                    let record = program.typedefs.get(&i.type_id).get_record();

                    let mut args = Vec::new();

                    let (func_type_var, result) = create_general_function_type(
                        record.fields.len(),
                        &mut args,
                        &mut self.type_store,
                    );

                    let record_type_info = self
                        .record_type_info_map
                        .get(&i.type_id)
                        .expect("record type info not found");

                    for (index, field) in record_type_info.field_types.iter().enumerate() {
                        let r = self.type_store.unify(&field.0, &args[index]);
                        assert!(r);
                    }
                    let r = self
                        .type_store
                        .unify(&result, &record_type_info.record_type);
                    assert!(r);
                    let type_info = FunctionTypeInfo::new(
                        format!("{}_ctor", record.name),
                        args.clone(),
                        true,
                        record_type_info.record_type,
                        func_type_var,
                        None,
                        record.location_id,
                        BTreeMap::new(),
                    );
                    self.function_type_info_map.insert(*id, type_info);
                }
                FunctionInfo::VariantConstructor(i) => {
                    let key = (i.type_id, i.index);
                    let adt = program.typedefs.get(&i.type_id).get_adt();
                    let variant_type_info = self
                        .variant_type_info_map
                        .get(&key)
                        .expect("variant type info not found");
                    let mut args = Vec::new();

                    let (func_type_var, result) = create_general_function_type(
                        variant_type_info.item_types.len(),
                        &mut args,
                        &mut self.type_store,
                    );
                    for (index, item_type) in variant_type_info.item_types.iter().enumerate() {
                        let r = self.type_store.unify(&item_type.0, &args[index]);
                        assert!(r);
                    }
                    let r = self
                        .type_store
                        .unify(&result, &variant_type_info.variant_type);
                    assert!(r);
                    let type_info = FunctionTypeInfo::new(
                        format!("{}/{}_ctor", adt.name, adt.variants[i.index].name),
                        args.clone(),
                        true,
                        variant_type_info.variant_type,
                        func_type_var,
                        None,
                        variant_type_info.location_id,
                        BTreeMap::new(),
                    );

                    self.function_type_info_map.insert(*id, type_info);
                }
                FunctionInfo::Lambda(i) => {
                    self.register_untyped_function(
                        displayed_name,
                        *id,
                        function,
                        i.body,
                        i.location_id,
                    );
                }
                FunctionInfo::NamedFunction(i) => match i.type_signature {
                    Some(type_signature) => {
                        self.register_typed_function(
                            displayed_name,
                            &i.name,
                            function.arg_locations.len(),
                            type_signature,
                            *id,
                            program,
                            errors,
                            i.body,
                            i.location_id,
                            &i.kind,
                            class_member_type_info_map,
                        );
                    }
                    None => match i.body {
                        Some(body) => {
                            self.register_untyped_function(
                                displayed_name,
                                *id,
                                function,
                                body,
                                i.location_id,
                            );
                        }
                        None => {
                            let err = TypecheckError::UntypedExternFunction(
                                i.name.clone(),
                                i.location_id,
                            );
                            errors.push(err)
                        }
                    },
                },
            }
        }

        self.function_type_info_map
    }
}
