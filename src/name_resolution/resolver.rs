use crate::error::Error;
use crate::ir::function::Function as IrFunction;
use crate::ir::function::FunctionId as IrFunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::function::NamedFunctionInfo;
use crate::ir::program::Program as IrProgram;
use crate::ir::types::Adt;
use crate::ir::types::Record;
use crate::ir::types::TypeDef;
use crate::ir::types::TypeInfo;
use crate::ir::types::TypeSignature as IrTypeSignature;
use crate::ir::types::TypeSignatureId as IrTypeSignatureId;
use crate::name_resolution::environment::Environment;
use crate::name_resolution::error::ResolverError;
use crate::name_resolution::export_processor::process_exports;
use crate::name_resolution::expr_processor::process_expr;
use crate::name_resolution::import_processor::process_imports;
use crate::name_resolution::item::Item;
use crate::name_resolution::lambda_helper::LambdaHelper;
use crate::name_resolution::module::Module;
use crate::syntax::function::FunctionBody as AstFunctionBody;
use crate::syntax::function::FunctionId as AstFunctionId;
use crate::syntax::function::FunctionType as AstFunctionType;
use crate::syntax::module::Module as AstModule;
use crate::syntax::program::Program;
use crate::syntax::types::TypeSignature as AstTypeSignature;
use crate::syntax::types::TypeSignatureId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug)]
pub struct Resolver {
    modules: BTreeMap<String, Module>,
    function_map: BTreeMap<AstFunctionId, IrFunctionId>,
}

impl Resolver {
    pub fn new() -> Resolver {
        Resolver {
            modules: BTreeMap::new(),
            function_map: BTreeMap::new(),
        }
    }

    fn register_module(
        &mut self,
        ast_module: &AstModule,
        modules: &mut BTreeMap<String, Vec<Module>>,
    ) {
        let mut module = Module::new(
            ast_module.id,
            ast_module.name.clone(),
            ast_module.location_id,
        );

        let mods = modules
            .entry(ast_module.name.get())
            .or_insert_with(Vec::new);
        mods.push(module);
    }

    fn process_module_conflicts(
        &mut self,
        modules: BTreeMap<String, Vec<Module>>,
    ) -> Result<(), Error> {
        let mut errors = Vec::new();
        let mut module_conflicts = BTreeMap::new();

        for (name, modules) in modules.iter() {
            if modules.len() > 1 {
                let ids = modules.iter().map(|m| m.location_id).collect();
                module_conflicts.insert(name.clone(), ids);
            }
        }

        if !module_conflicts.is_empty() {
            let e = ResolverError::ModuleConflict(module_conflicts);
            errors.push(e);
        }

        if errors.is_empty() {
            for (name, mods) in modules {
                let modules: Vec<Module> = mods;
                self.modules
                    .insert(name, modules.into_iter().next().expect("Empty module set"));
            }
            Ok(())
        } else {
            return Err(Error::resolve_err(errors));
        }
    }

    /*
    fn collect_imported_symbols(
        &self,
        ast_import: &AstImport,
        source_module: &Module,
    ) -> (ImportStore, Vec<ResolverError>) {
        let mut import_store = ImportStore::new();
        let mut errors = Vec::new();
        /*
        let (namespace, kind) = match &ast_import.alternative_name {
            Some(n) => (n.clone(), ImportKind::NamespaceOnly),
            None => (ast_import.module_path.get(), ImportKind::NameAndNamespace),
        };
        match &ast_import.kind {
            AstImportKind::Explicit(imported_items) => {
                for imported_item in imported_items {
                    let item_name = if let AstImportedItem::FunctionOrRecord(name) = imported_item {
                        name.clone()
                    } else {
                        unimplemented!()
                    };
                    match source_module.exported_functions.get(&item_name) {
                        Some(_) => import_store.add_imported_function(
                            item_name.clone(),
                            source_module.name.clone(),
                            namespace.clone(),
                            kind,
                        ),
                        None => {
                            let e = ResolverError::SymbolNotFoundInModule(
                                item_name.clone(),
                                ast_import.id.clone(),
                            );
                            errors.push(e);
                        }
                    }
                }
            }
            AstImportKind::ImplicitAll => {
                for func in source_module.exported_functions.keys() {
                    import_store.add_imported_function(
                        func.clone(),
                        source_module.name.clone(),
                        namespace.clone(),
                        kind,
                    );
                }
            }
            AstImportKind::Hiding(_) => unimplemented!(),
        }
        */
    (import_store, errors)
    }
     */

    fn process_type_signature(
        &self,
        type_signature_id: &TypeSignatureId,
        program: &Program,
        ir_program: &mut IrProgram,
        type_args: &BTreeMap<String, usize>,
        errors: &mut Vec<ResolverError>,
        used_type_args: &mut BTreeSet<String>,
    ) -> Option<IrTypeSignatureId> {
        let type_signature = program.get_type_signature(type_signature_id);
        let location_id = program.get_type_signature_location(type_signature_id);
        let ir_type_signature = match type_signature {
            AstTypeSignature::Nothing => IrTypeSignature::Nothing,
            AstTypeSignature::Named(n, _) => match n.get().as_ref() {
                "Int" => IrTypeSignature::Int,
                "Bool" => IrTypeSignature::Bool,
                "String" => IrTypeSignature::String,
                _ => {
                    if let Some(index) = type_args.get(&n.get()) {
                        used_type_args.insert(n.get().clone());
                        IrTypeSignature::TypeArgument(*index)
                    } else {
                        let error = ResolverError::UnknownTypeName(n.get().clone(), location_id);
                        errors.push(error);
                        return None;
                    }
                }
            },
            AstTypeSignature::Tuple(items) => {
                let mut item_ids = Vec::new();
                for item in items {
                    match self.process_type_signature(
                        item,
                        program,
                        ir_program,
                        type_args,
                        errors,
                        used_type_args,
                    ) {
                        Some(id) => {
                            item_ids.push(id);
                        }
                        None => {
                            return None;
                        }
                    }
                }
                IrTypeSignature::Tuple(item_ids)
            }
            AstTypeSignature::Function(items) => {
                let mut item_ids = Vec::new();
                for item in items {
                    match self.process_type_signature(
                        item,
                        program,
                        ir_program,
                        type_args,
                        errors,
                        used_type_args,
                    ) {
                        Some(id) => {
                            item_ids.push(id);
                        }
                        None => {
                            return None;
                        }
                    }
                }
                IrTypeSignature::Function(item_ids)
            }
            AstTypeSignature::TypeArgument(_) => unimplemented!(),
        };
        let id = ir_program.get_type_signature_id();
        let type_info = TypeInfo::new(ir_type_signature, type_signature_id.clone());
        ir_program.add_type_signature(id, type_info);
        return Some(id);
    }

    fn process_func_type(
        &self,
        func_type: &AstFunctionType,
        program: &Program,
        ir_program: &mut IrProgram,
        errors: &mut Vec<ResolverError>,
    ) -> Option<IrTypeSignatureId> {
        let mut type_args = BTreeMap::new();
        let mut conflicting_names = BTreeSet::new();
        let location_id = func_type.location_id;
        for (index, type_arg) in func_type.type_args.iter().enumerate() {
            if type_args.insert(type_arg.clone(), index).is_some() {
                conflicting_names.insert(type_arg.clone());
            }
        }
        if !conflicting_names.is_empty() {
            let error = ResolverError::TypeArgumentConflict(
                conflicting_names.iter().cloned().collect(),
                location_id,
            );
            errors.push(error);
        }

        let mut used_type_args = BTreeSet::new();

        let id = self.process_type_signature(
            &func_type.type_signature_id,
            program,
            ir_program,
            &type_args,
            errors,
            &mut used_type_args,
        );

        let mut unused = Vec::new();
        for type_arg in type_args.keys() {
            if !used_type_args.contains(type_arg) {
                unused.push(type_arg.clone());
            }
        }

        if !unused.is_empty() {
            let err = ResolverError::UnusedTypeArgument(unused, location_id);
            errors.push(err);
        }

        id
    }

    fn resolve_named_function_id(&self, named_id: &(String, String)) -> IrFunctionId {
        /*
        let m = self.modules.get(&named_id.0).expect("Module not found");
        let f = m
            .exported_functions
            .get(&named_id.1)
            .expect("Function not found");
        let ast_id = f[0].id.clone();
        let ir_function_id = self
            .function_map
            .get(&ast_id)
            .expect("Ir function not found");
        ir_function_id.clone()
        */
        unreachable!()
    }

    fn process_items_and_types(
        &mut self,
        program: &Program,
        errors: &mut Vec<ResolverError>,
        ir_program: &mut IrProgram,
    ) {
        for (name, module) in &mut self.modules {
            let ast_module = program.modules.get(&module.id).expect("Module not found");
            for record_id in &ast_module.records {
                let record = program.records.get(record_id).expect("Record not found");
                let items = module
                    .items
                    .entry(record.name.clone())
                    .or_insert_with(|| Vec::new());
                items.push(Item::Record(*record_id));
                let ir_typedef_id = ir_program.get_typedef_id();
                let ir_record = Record {
                    name: record.name.clone(),
                    ast_record_id: *record_id,
                    id: ir_typedef_id,
                };
                let typedef = TypeDef::Record(ir_record);
                ir_program.add_typedef(ir_typedef_id, typedef);
            }
            for adt_id in &ast_module.adts {
                let adt = program.adts.get(adt_id).expect("Adt not found");
                let items = module
                    .items
                    .entry(adt.name.clone())
                    .or_insert_with(|| Vec::new());
                items.push(Item::Adt(*adt_id));
                let ir_typedef_id = ir_program.get_typedef_id();
                let ir_adt = Adt {
                    name: adt.name.clone(),
                    ast_adt_id: *adt_id,
                    id: ir_typedef_id,
                };
                let typedef = TypeDef::Adt(ir_adt);
                ir_program.add_typedef(ir_typedef_id, typedef);
            }
            for function_id in &ast_module.functions {
                let function = program
                    .functions
                    .get(function_id)
                    .expect("Function not found");
                let items = module
                    .items
                    .entry(function.name.clone())
                    .or_insert_with(|| Vec::new());
                items.push(Item::Function(function.id));
                let ir_function_id = ir_program.get_function_id();
                self.function_map.insert(*function_id, ir_function_id);
            }
        }

        for (_, module) in &self.modules {
            for (name, items) in &module.items {
                if items.len() > 1 {
                    let mut locations = Vec::new();
                    for item in items {
                        match item {
                            Item::Function(id) => {
                                let function =
                                    program.functions.get(id).expect("Function not found");
                                locations.push(function.location_id);
                            }
                            Item::Record(id) => {
                                let record = program.records.get(id).expect("Record not found");
                                locations.push(record.location_id);
                            }
                            Item::Adt(id) => {
                                let adt = program.adts.get(id).expect("Adt not found");
                                locations.push(adt.location_id);
                            }
                        }
                    }
                    let err = ResolverError::InternalModuleConflicts(
                        module.name.get(),
                        name.clone(),
                        locations,
                    );
                    errors.push(err);
                }
            }
        }

        for (_, record) in &program.records {
            if record.name != record.data_name {
                let err = ResolverError::RecordTypeNameMismatch(
                    record.name.clone(),
                    record.data_name.clone(),
                    record.location_id,
                );
                errors.push(err);
            }
            let mut field_names = BTreeSet::new();
            for field in &record.fields {
                if !field_names.insert(field.name.clone()) {
                    let err = ResolverError::RecordFieldNotUnique(
                        record.name.clone(),
                        field.name.clone(),
                        record.location_id,
                    );
                    errors.push(err);
                }
            }
        }

        for (_, adt) in &program.adts {
            let mut variant_names = BTreeSet::new();
            for variant_id in &adt.variants {
                let variant = program.variants.get(variant_id).expect("Variant not found");
                if !variant_names.insert(variant.name.clone()) {
                    let err = ResolverError::VariantNotUnique(
                        adt.name.clone(),
                        variant.name.clone(),
                        adt.location_id,
                    );
                    errors.push(err);
                }
            }
        }
    }

    pub fn resolve(&mut self, program: &Program) -> Result<IrProgram, Error> {
        let mut errors = Vec::new();

        let mut modules = BTreeMap::new();

        for ast_module in program.modules.values() {
            self.register_module(ast_module, &mut modules);
        }

        self.process_module_conflicts(modules)?;

        let mut ir_program = IrProgram::new();

        self.process_items_and_types(program, &mut errors, &mut ir_program);

        if !errors.is_empty() {
            return Err(Error::resolve_err(errors));
        }

        process_exports(&mut self.modules, program, &mut errors);

        if !errors.is_empty() {
            return Err(Error::resolve_err(errors));
        }

        process_imports(&mut self.modules, program, &mut errors);

        if !errors.is_empty() {
            return Err(Error::resolve_err(errors));
        }

        for (_, module) in &program.modules {
            for function_id in &module.functions {
                let function = program
                    .functions
                    .get(function_id)
                    .expect("Function not found");
                let resolver_module = self
                    .modules
                    .get(&module.name.get())
                    .expect("Resolver module not found");
                let ir_function_id = self
                    .function_map
                    .get(&function.id)
                    .expect("Function not found")
                    .clone();
                let mut type_signature_id = None;
                let mut body = None;
                if let Some(ty) = &function.func_type {
                    if ty.name != function.name {
                        let err = ResolverError::FunctionTypeNameMismatch(
                            ty.name.clone(),
                            function.name.clone(),
                            ty.location_id,
                        );
                        errors.push(err);
                    }
                    type_signature_id =
                        self.process_func_type(ty, program, &mut ir_program, &mut errors);
                }
                if let AstFunctionBody::Expr(id) = function.body {
                    let mut environment = Environment::new();
                    let mut arg_names = BTreeSet::new();
                    let mut conflicting_names = BTreeSet::new();
                    for (index, arg) in function.args.iter().enumerate() {
                        if !arg_names.insert(arg.clone()) {
                            conflicting_names.insert(arg.clone());
                        }
                        environment.add_arg(arg.clone(), ir_function_id, index);
                    }
                    if !conflicting_names.is_empty() {
                        let err = ResolverError::ArgumentConflict(
                            conflicting_names.into_iter().collect(),
                            function.location_id.clone(),
                        );
                        errors.push(err);
                    }
                    let host_function = format!("{}/{}", module.name.get(), function.name);
                    let mut lambda_helper = LambdaHelper::new(
                        0,
                        host_function,
                        LambdaHelper::new_counter(),
                        ir_function_id,
                    );
                    let body_id = process_expr(
                        id,
                        program,
                        resolver_module,
                        &mut environment,
                        &mut ir_program,
                        &mut errors,
                        &mut lambda_helper,
                    );
                    body = Some(body_id);
                }

                let named_info = NamedFunctionInfo {
                    body: body,
                    name: function.name.clone(),
                    module: module.name.get(),
                    type_signature: type_signature_id,
                    ast_function_id: function.id,
                    location_id: function.location_id,
                };

                let ir_function = IrFunction {
                    id: ir_function_id,
                    arg_count: function.args.len(),
                    info: FunctionInfo::NamedFunction(named_info),
                };
                ir_program.add_function(ir_function_id, ir_function);
            }
        }

        if !errors.is_empty() {
            return Err(Error::resolve_err(errors));
        }

        Ok(ir_program)
    }
}
