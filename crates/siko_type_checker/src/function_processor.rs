use crate::common::create_general_function_type;
use crate::common::FunctionTypeInfo;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::error::TypecheckError;
use crate::type_processor::process_type_signature;
use crate::type_store::TypeStore;
use crate::types::Type;
use siko_ir::expr::ExprId;
use siko_ir::function::Function;
use siko_ir::function::FunctionId;
use siko_ir::function::FunctionInfo;
use siko_ir::program::Program;
use siko_ir::types::TypeDefId;
use siko_ir::types::TypeSignatureId;
use siko_location_info::item::LocationId;
use std::collections::BTreeMap;

pub struct FunctionProcessor {
    type_store: TypeStore,
    function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    record_type_info_map: BTreeMap<TypeDefId, RecordTypeInfo>,
    variant_type_info_map: BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
}

impl FunctionProcessor {
    pub fn new() -> FunctionProcessor {
        FunctionProcessor {
            type_store: TypeStore::new(),
            function_type_info_map: BTreeMap::new(),
            record_type_info_map: BTreeMap::new(),
            variant_type_info_map: BTreeMap::new(),
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
    ) {
        let mut arg_map = BTreeMap::new();
        let func_type_var = process_type_signature(
            &mut self.type_store,
            &type_signature_id,
            program,
            &mut arg_map,
        );
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
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        name.clone(),
                        arg_count,
                        signature_vars.len(),
                        program.type_signatures.get(&type_signature_id).location_id,
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
                )
            }
            _ => {
                if arg_count > 0 {
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        displayed_name,
                        arg_count,
                        0,
                        program.type_signatures.get(&type_signature_id).location_id,
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
        );
        self.function_type_info_map.insert(id, function_type_info);
    }

    pub fn process_functions(
        mut self,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) -> (
        TypeStore,
        BTreeMap<FunctionId, FunctionTypeInfo>,
        BTreeMap<TypeDefId, RecordTypeInfo>,
        BTreeMap<(TypeDefId, usize), VariantTypeInfo>,
    ) {
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

                    let mut arg_map = BTreeMap::new();
                    for (index, field) in record.fields.iter().enumerate() {
                        let arg_var = process_type_signature(
                            &mut self.type_store,
                            &field.type_signature_id,
                            program,
                            &mut arg_map,
                        );
                        let r = self.type_store.unify(&arg_var, &args[index]);
                        assert!(r);
                    }
                    let mut type_args: Vec<_> = Vec::new();
                    for arg in &record.type_args {
                        let var = match arg_map.get(arg) {
                            Some(v) => *v,
                            None => self.type_store.get_new_type_var(),
                        };
                        type_args.push(var);
                    }
                    let result_type = Type::Named(record.name.clone(), i.type_id, type_args);
                    let result_type_var = self.type_store.add_type(result_type);
                    let r = self.type_store.unify(&result, &result_type_var);
                    assert!(r);
                    let type_info = FunctionTypeInfo::new(
                        format!("{}_ctor", record.name),
                        args.clone(),
                        true,
                        result_type_var,
                        func_type_var,
                        None,
                        record.location_id,
                    );
                    let record_type_info = RecordTypeInfo {
                        record_type: result_type_var,
                        field_types: args,
                    };
                    self.record_type_info_map
                        .insert(record.id, record_type_info);
                    self.function_type_info_map.insert(*id, type_info);
                }
                FunctionInfo::VariantConstructor(i) => {
                    let adt = program.typedefs.get(&i.type_id).get_adt();
                    let variant = &adt.variants[i.index];
                    let location_id = program
                        .type_signatures
                        .get(&variant.type_signature_id)
                        .location_id;

                    let mut args = Vec::new();

                    let (func_type_var, result) = create_general_function_type(
                        variant.items.len(),
                        &mut args,
                        &mut self.type_store,
                    );

                    let mut arg_map = BTreeMap::new();
                    for (index, item) in variant.items.iter().enumerate() {
                        let arg_var = process_type_signature(
                            &mut self.type_store,
                            &item.type_signature_id,
                            program,
                            &mut arg_map,
                        );
                        let r = self.type_store.unify(&arg_var, &args[index]);
                        assert!(r);
                    }
                    let mut type_args: Vec<_> = Vec::new();
                    for arg in &adt.type_args {
                        let var = match arg_map.get(arg) {
                            Some(v) => *v,
                            None => self.type_store.get_new_type_var(),
                        };
                        type_args.push(var);
                    }
                    let result_type = Type::Named(adt.name.clone(), i.type_id, type_args);
                    let result_type_var = self.type_store.add_type(result_type);
                    let r = self.type_store.unify(&result, &result_type_var);
                    assert!(r);
                    let type_info = FunctionTypeInfo::new(
                        format!("{}/{}_ctor", adt.name, variant.name),
                        args.clone(),
                        true,
                        result_type_var,
                        func_type_var,
                        None,
                        location_id,
                    );

                    let variant_type_info = VariantTypeInfo {
                        variant_type: result_type_var,
                        item_types: args,
                    };

                    self.variant_type_info_map
                        .insert((i.type_id, i.index), variant_type_info);

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

        (
            self.type_store,
            self.function_type_info_map,
            self.record_type_info_map,
            self.variant_type_info_map,
        )
    }
}
