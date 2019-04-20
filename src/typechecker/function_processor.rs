use crate::ir::expr::ExprId;
use crate::ir::function::Function;
use crate::ir::function::FunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::function::NamedFunctionInfo;
use crate::ir::program::Program;
use crate::ir::types::TypeSignature;
use crate::ir::types::TypeSignatureId;
use crate::location_info::item::LocationId;
use crate::typechecker::common::create_general_function_type;
use crate::typechecker::common::FunctionSignatureLocation;
use crate::typechecker::common::FunctionTypeInfo;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use std::collections::BTreeMap;

pub struct FunctionProcessor {
    pub type_store: TypeStore,
    pub function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
}

impl FunctionProcessor {
    pub fn new() -> FunctionProcessor {
        FunctionProcessor {
            type_store: TypeStore::new(),
            function_type_info_map: BTreeMap::new(),
        }
    }

    fn process_type_signature(
        &mut self,
        type_signature_id: &TypeSignatureId,
        program: &Program,
        arg_map: &mut BTreeMap<usize, TypeVariable>,
        signature_arg_locations: &mut Vec<LocationId>,
        arg_count: usize,
    ) -> (TypeVariable, LocationId) {
        let type_signature = program.get_type_signature(type_signature_id);
        let location_id = program.get_type_signature_location(type_signature_id);
        match type_signature {
            TypeSignature::Bool => {
                let ty = Type::Bool;
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::Int => {
                let ty = Type::Int;
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::String => {
                let ty = Type::String;
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::Nothing => {
                let ty = Type::Nothing;
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::Tuple(items) => {
                let items: Vec<_> = items
                    .iter()
                    .map(|i| {
                        self.process_type_signature(
                            i,
                            program,
                            arg_map,
                            signature_arg_locations,
                            arg_count,
                        )
                    })
                    .map(|(i, _)| i)
                    .collect();
                let ty = Type::Tuple(items);
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::Function(from, to) => {
                let (from_var, _) =
                    self.process_type_signature(from, program, arg_map, signature_arg_locations, 0);
                let (to_var, _) = self.process_type_signature(
                    to,
                    program,
                    arg_map,
                    signature_arg_locations,
                    if arg_count > 0 { arg_count - 1 } else { 0 },
                );
                if arg_count > 0 {
                    let from_location_id = program.get_type_signature_location(from);
                    signature_arg_locations.push(from_location_id);
                }
                let to_location_id = program.get_type_signature_location(to);
                let ty = Type::Function(FunctionType::new(from_var, to_var));
                return (self.type_store.add_type(ty), to_location_id);
            }
            TypeSignature::TypeArgument(index, name) => {
                let var = arg_map.entry(*index).or_insert_with(|| {
                    let arg = self.type_store.get_unique_type_arg();
                    let ty = Type::FixedTypeArgument(arg, name.clone());
                    self.type_store.add_type(ty)
                });
                return (*var, location_id);
            }
            TypeSignature::Named(..) => {
                let ty = Type::Nothing;
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::Variant(..) => {
                let ty = Type::Nothing;
                return (self.type_store.add_type(ty), location_id);
            }
        }
    }

    fn register_typed_function(
        &mut self,
        displayed_name: String,
        named_info: &NamedFunctionInfo,
        arg_locations: Vec<LocationId>,
        type_signature_id: TypeSignatureId,
        function_id: FunctionId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
        body: Option<ExprId>,
    ) {
        let mut arg_map = BTreeMap::new();
        let mut signature_arg_locations = Vec::new();
        let (func_type_var, return_location_id) = self.process_type_signature(
            &type_signature_id,
            program,
            &mut arg_map,
            &mut signature_arg_locations,
            arg_locations.len(),
        );
        /*
        println!(
            "Registering named function {} {} with type {}",
            function_id,
            displayed_name,
            self.type_store.get_resolved_type_string(&func_type_var)
        );
        */
        let ty = self.type_store.get_type(&func_type_var);
        let function_type_info = match ty {
            Type::Function(func_type) => {
                let mut signature_vars = Vec::new();
                func_type.get_arg_types(&self.type_store, &mut signature_vars);
                if signature_vars.len() < arg_locations.len() {
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        named_info.name.clone(),
                        arg_locations.len(),
                        signature_vars.len(),
                        program.get_type_signature_location(&type_signature_id),
                    );
                    errors.push(err);
                    return;
                }
                let signature_location = FunctionSignatureLocation {
                    return_location_id: return_location_id,
                    arg_locations: signature_arg_locations,
                };

                let arg_vars: Vec<_> = signature_vars
                    .iter()
                    .take(arg_locations.len())
                    .cloned()
                    .collect();

                let return_value_var = func_type.get_return_type(&self.type_store, arg_vars.len());
                FunctionTypeInfo::new(
                    displayed_name,
                    arg_vars,
                    Some(signature_location),
                    arg_locations,
                    return_value_var,
                    func_type_var,
                    body,
                )
            }
            _ => {
                if arg_locations.len() > 0 {
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        displayed_name,
                        arg_locations.len(),
                        0,
                        program.get_type_signature_location(&type_signature_id),
                    );
                    errors.push(err);
                    return;
                }
                let signature_location = FunctionSignatureLocation {
                    return_location_id: return_location_id,
                    arg_locations: vec![],
                };
                FunctionTypeInfo::new(
                    named_info.name.clone(),
                    vec![],
                    Some(signature_location),
                    arg_locations,
                    func_type_var,
                    func_type_var,
                    body,
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
    ) {
        let mut args = Vec::new();

        let (func_type_var, result) = create_general_function_type(
            function.arg_locations.len(),
            &mut args,
            &mut self.type_store,
        );
        let function_type_info = FunctionTypeInfo::new(
            name,
            args,
            None,
            function.arg_locations.clone(),
            result,
            func_type_var,
            Some(body),
        );
        self.function_type_info_map.insert(id, function_type_info);
    }

    pub fn process_functions(
        mut self,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) -> (TypeStore, BTreeMap<FunctionId, FunctionTypeInfo>) {
        for (id, function) in &program.functions {
            let displayed_name = format!("{}", function.info);
            match &function.info {
                FunctionInfo::RecordConstructor(_) => {}
                FunctionInfo::VariantConstructor(_) => {}
                FunctionInfo::Lambda(_) => {}
                FunctionInfo::NamedFunction(i) => match i.type_signature {
                    Some(type_signature) => {
                        self.register_typed_function(
                            displayed_name,
                            i,
                            function.arg_locations.clone(),
                            type_signature,
                            *id,
                            program,
                            errors,
                            i.body,
                        );
                    }
                    None => match i.body {
                        Some(body) => {
                            self.register_untyped_function(displayed_name, *id, function, body);
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
        (self.type_store, self.function_type_info_map)
    }
}
