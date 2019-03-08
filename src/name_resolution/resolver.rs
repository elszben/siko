use crate::constants::BuiltinOperator;
use crate::constants::PRELUDE_NAME;
use crate::error::Error;
use crate::ir::expr::Expr as IrExpr;
use crate::ir::expr::ExprId as IrExprId;
use crate::ir::expr::ExprInfo as IrExprInfo;
use crate::ir::function::Function as IrFunction;
use crate::ir::function::FunctionId as IrFunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::function::LambdaInfo;
use crate::ir::function::NamedFunctionInfo;
use crate::ir::program::Program as IrProgram;
use crate::ir::types::TypeInfo;
use crate::ir::types::TypeSignature as IrTypeSignature;
use crate::ir::types::TypeSignatureId as IrTypeSignatureId;
use crate::name_resolution::environment::Environment;
use crate::name_resolution::environment::NamedRef;
use crate::name_resolution::error::ResolverError;
use crate::name_resolution::import::ImportKind;
use crate::name_resolution::import::ImportStore;
use crate::name_resolution::lambda_helper::LambdaHelper;
use crate::name_resolution::module::Module;
use crate::syntax::expr::Expr;
use crate::syntax::expr::ExprId;
use crate::syntax::function::FunctionBody as AstFunctionBody;
use crate::syntax::function::FunctionId as AstFunctionId;
use crate::syntax::function::FunctionType as AstFunctionType;
use crate::syntax::import::Import as AstImport;
use crate::syntax::item_path::ItemPath;
use crate::syntax::module::Module as AstModule;
use crate::syntax::program::Program;
use crate::syntax::types::TypeSignature as AstTypeSignature;
use crate::syntax::types::TypeSignatureId;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

enum PathResolveResult {
    VariableRef(NamedRef),
    FunctionRef(IrFunctionId),
    Unknown(String),
    Ambiguous,
}

#[derive(Debug)]
pub struct Resolver<'a> {
    modules: BTreeMap<String, Module<'a>>,
    function_map: BTreeMap<AstFunctionId, IrFunctionId>,
}

impl<'a> Resolver<'a> {
    pub fn new() -> Resolver<'a> {
        Resolver {
            modules: BTreeMap::new(),
            function_map: BTreeMap::new(),
        }
    }

    fn register_module<'b>(
        &mut self,
        ast_module: &'b AstModule,
        modules: &mut BTreeMap<String, Vec<Module<'b>>>,
        ir_program: &mut IrProgram,
    ) {
        let mut module = Module::new(ast_module.id, ast_module.name.clone());

        for function in ast_module.functions.values() {
            let functions = module
                .exported_functions
                .entry(function.name.clone())
                .or_insert_with(Vec::new);
            functions.push(function);
            module.imported_functions.add_imported_function(
                function.name.clone(),
                ast_module.name.clone(),
                String::new(),
                ImportKind::NameOnly,
            );
            self.function_map
                .insert(function.id.clone(), ir_program.get_function_id());
        }

        let mods = modules
            .entry(ast_module.name.get())
            .or_insert_with(Vec::new);
        mods.push(module);
    }

    fn process_module_and_export_conflicts(
        &mut self,
        modules: BTreeMap<String, Vec<Module<'a>>>,
    ) -> Result<(), Error> {
        let mut errors = Vec::new();
        let mut module_conflicts = BTreeMap::new();

        for (name, modules) in modules.iter() {
            if modules.len() > 1 {
                let ids = modules.iter().map(|m| m.id).collect();
                module_conflicts.insert(name.clone(), ids);
            }
        }

        if !module_conflicts.is_empty() {
            let e = ResolverError::ModuleConflict(module_conflicts);
            errors.push(e);
        }

        if errors.is_empty() {
            for (name, mods) in modules {
                let modules: Vec<Module<'a>> = mods;
                self.modules
                    .insert(name, modules.into_iter().next().expect("Empty module set"));
            }
            Ok(())
        } else {
            return Err(Error::resolve_err(errors));
        }
    }

    fn collect_imported_symbols(
        &self,
        ast_import: &AstImport,
        source_module: &Module,
    ) -> (ImportStore, Vec<ResolverError>) {
        let mut import_store = ImportStore::new();
        let mut errors = Vec::new();
        let (namespace, kind) = match &ast_import.alternative_name {
            Some(n) => (n.clone(), ImportKind::NamespaceOnly),
            None => (ast_import.module_path.get(), ImportKind::NameAndNamespace),
        };
        match &ast_import.symbols {
            Some(symbols) => {
                for symbol in symbols {
                    match source_module.exported_functions.get(symbol) {
                        Some(_) => import_store.add_imported_function(
                            symbol.clone(),
                            source_module.name.clone(),
                            namespace.clone(),
                            kind,
                        ),
                        None => {
                            let e = ResolverError::SymbolNotFoundInModule(
                                symbol.clone(),
                                ast_import.id.clone(),
                            );
                            errors.push(e);
                        }
                    }
                }
            }
            None => {
                for func in source_module.exported_functions.keys() {
                    import_store.add_imported_function(
                        func.clone(),
                        source_module.name.clone(),
                        namespace.clone(),
                        kind,
                    );
                }
            }
        }
        (import_store, errors)
    }

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
        let ir_type_signature = match type_signature {
            AstTypeSignature::Nothing => IrTypeSignature::Nothing,
            AstTypeSignature::Named(n) => match n.as_ref() {
                "Int" => IrTypeSignature::Int,
                "Bool" => IrTypeSignature::Bool,
                "String" => IrTypeSignature::String,
                _ => {
                    if let Some(index) = type_args.get(n) {
                        used_type_args.insert(n.clone());
                        IrTypeSignature::TypeArgument(*index)
                    } else {
                        let error =
                            ResolverError::UnknownTypeName(n.clone(), type_signature_id.clone());
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
        for (index, type_arg) in func_type.type_args.iter().enumerate() {
            if type_args.insert(type_arg.clone(), index).is_some() {
                conflicting_names.insert(type_arg.clone());
            }
        }
        if !conflicting_names.is_empty() {
            let error = ResolverError::TypeArgumentConflict(
                conflicting_names.iter().cloned().collect(),
                func_type.full_type_signature_id.clone(),
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
            let err = ResolverError::UnusedTypeArgument(unused, func_type.full_type_signature_id);
            errors.push(err);
        }

        id
    }

    fn add_expr(&self, ir_expr: IrExpr, ast_id: ExprId, ir_program: &mut IrProgram) -> IrExprId {
        let expr_id = ir_program.get_expr_id();
        let expr_info = IrExprInfo::new(ir_expr, ast_id);
        ir_program.add_expr(expr_id, expr_info);
        expr_id
    }

    fn resolve_named_function_id(&self, named_id: &(String, String)) -> IrFunctionId {
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
    }

    fn resolve_item_path(
        &self,
        path: &ItemPath,
        module: &Module<'a>,
        environment: &Environment,
        lambda_helper: &mut LambdaHelper,
    ) -> PathResolveResult {
        let name = path.get();
        if path.path.len() == 1 {
            if let Some((named_ref, level)) = environment.get_ref(&name) {
                let named_ref = lambda_helper.process_named_ref(named_ref.clone(), level);
                return PathResolveResult::VariableRef(named_ref);
            }
        }
        let function_ids = module.imported_functions.get_function_id(&name);
        match function_ids.len() {
            0 => {
                return PathResolveResult::Unknown(name);
            }
            1 => {
                let id = self.resolve_named_function_id(&function_ids[0]);
                return PathResolveResult::FunctionRef(id);
            }
            _ => {
                return PathResolveResult::Ambiguous;
            }
        }
    }

    fn process_named_ref(
        &self,
        named_ref: NamedRef,
        id: ExprId,
        ir_program: &mut IrProgram,
    ) -> IrExprId {
        let ir_expr = match named_ref {
            NamedRef::ExprValue(expr_ref) => IrExpr::ExprValue(expr_ref),
            NamedRef::FunctionArg(arg_ref) => IrExpr::ArgRef(arg_ref),
            NamedRef::LambdaCapturedExprValue(_, arg_ref) => IrExpr::LambdaCapturedArgRef(arg_ref),
            NamedRef::LambdaCapturedFunctionArg(_, arg_ref) => {
                IrExpr::LambdaCapturedArgRef(arg_ref)
            }
        };
        self.add_expr(ir_expr, id, ir_program)
    }

    fn process_expr(
        &self,
        id: ExprId,
        program: &Program,
        module: &Module<'a>,
        environment: &mut Environment,
        ir_program: &mut IrProgram,
        errors: &mut Vec<ResolverError>,
        lambda_helper: &mut LambdaHelper,
    ) -> IrExprId {
        let expr = program.get_expr(&id);
        //println!("Processing expr {}", expr);
        match expr {
            Expr::Lambda(args, lambda_body) => {
                let ir_lambda_id = ir_program.get_function_id();
                let mut arg_names = BTreeSet::new();
                let mut conflicting_names = BTreeSet::new();
                let mut environment = Environment::child(environment);
                for (index, arg) in args.iter().enumerate() {
                    if !arg_names.insert(arg.clone()) {
                        conflicting_names.insert(arg.clone());
                    }
                    environment.add_arg(arg.clone(), ir_lambda_id, index);
                }
                if !conflicting_names.is_empty() {
                    let err = ResolverError::LambdaArgumentConflict(
                        conflicting_names.into_iter().collect(),
                        id.clone(),
                    );
                    errors.push(err);
                }
                let mut local_lambda_helper = LambdaHelper::new(
                    environment.level(),
                    lambda_helper.host_function(),
                    lambda_helper.clone_counter(),
                    ir_lambda_id,
                );

                let ir_lambda_body = self.process_expr(
                    *lambda_body,
                    program,
                    module,
                    &mut environment,
                    ir_program,
                    errors,
                    &mut local_lambda_helper,
                );

                let lambda_info = LambdaInfo {
                    body: ir_lambda_body,
                    host_info: local_lambda_helper.host_function(),
                    index: local_lambda_helper.get_lambda_index(),
                };

                let ir_function = IrFunction {
                    id: ir_lambda_id,
                    arg_count: args.len(),
                    info: FunctionInfo::Lambda(lambda_info),
                };
                ir_program.add_function(ir_lambda_id, ir_function);

                let captured_lambda_args: Vec<_> = local_lambda_helper
                    .captures()
                    .into_iter()
                    .map(|named_ref| self.process_named_ref(named_ref, id, ir_program))
                    .collect();
                let ir_expr = IrExpr::LambdaFunction(ir_lambda_id, captured_lambda_args);
                return self.add_expr(ir_expr, id, ir_program);
            }
            Expr::FunctionCall(id_expr_id, args) => {
                let ir_args: Vec<IrExprId> = args
                    .iter()
                    .map(|id| {
                        self.process_expr(
                            *id,
                            program,
                            module,
                            environment,
                            ir_program,
                            errors,
                            lambda_helper,
                        )
                    })
                    .collect();
                let id_expr = program.get_expr(id_expr_id);
                if let Expr::Path(path) = id_expr {
                    match self.resolve_item_path(path, module, environment, lambda_helper) {
                        PathResolveResult::FunctionRef(n) => {
                            let ir_expr = IrExpr::StaticFunctionCall(n, ir_args);
                            return self.add_expr(ir_expr, id, ir_program);
                        }
                        PathResolveResult::VariableRef(named_ref) => {
                            let ir_id_expr_id =
                                self.process_named_ref(named_ref, *id_expr_id, ir_program);
                            let ir_expr = IrExpr::DynamicFunctionCall(ir_id_expr_id, ir_args);
                            return self.add_expr(ir_expr, id, ir_program);
                        }
                        PathResolveResult::Unknown(n) => {
                            let err = ResolverError::UnknownFunction(n, id);
                            errors.push(err);
                            let ir_expr = IrExpr::Tuple(vec![]);
                            return self.add_expr(ir_expr, id, ir_program);
                        }
                        PathResolveResult::Ambiguous => {
                            let err = ResolverError::AmbiguousName(path.get(), id);
                            errors.push(err);
                            let ir_expr = IrExpr::Tuple(vec![]);
                            return self.add_expr(ir_expr, id, ir_program);
                        }
                    }
                } else {
                    if let Expr::Builtin(op) = id_expr {
                        if *op == BuiltinOperator::PipeForward {
                            assert_eq!(ir_args.len(), 2);
                            let left = ir_args[0];
                            let right = ir_args[0];
                            let ir_expr = IrExpr::DynamicFunctionCall(right, vec![left]);
                            return self.add_expr(ir_expr, id, ir_program);
                        } else {
                            let path = ItemPath {
                                path: vec![format!(
                                    "{}.op_{}",
                                    PRELUDE_NAME,
                                    format!("{:?}", op).to_lowercase()
                                )],
                            };
                            match self.resolve_item_path(&path, module, environment, lambda_helper)
                            {
                                PathResolveResult::FunctionRef(n) => {
                                    let ir_expr = IrExpr::StaticFunctionCall(n, ir_args);
                                    return self.add_expr(ir_expr, id, ir_program);
                                }
                                _ => panic!(
                                    "Couldn't handle builtin function {}, missing {}?",
                                    path.get(),
                                    PRELUDE_NAME
                                ),
                            }
                        }
                    } else {
                        let id_expr = self.process_expr(
                            *id_expr_id,
                            program,
                            module,
                            environment,
                            ir_program,
                            errors,
                            lambda_helper,
                        );
                        let ir_expr = IrExpr::DynamicFunctionCall(id_expr, ir_args);
                        return self.add_expr(ir_expr, id, ir_program);
                    }
                }
            }
            Expr::Builtin(_) => panic!("Builtinop reached!"),
            Expr::If(cond, true_branch, false_branch) => {
                let ir_cond = self.process_expr(
                    *cond,
                    program,
                    module,
                    environment,
                    ir_program,
                    errors,
                    lambda_helper,
                );
                let ir_true_branch = self.process_expr(
                    *true_branch,
                    program,
                    module,
                    environment,
                    ir_program,
                    errors,
                    lambda_helper,
                );
                let ir_false_branch = self.process_expr(
                    *false_branch,
                    program,
                    module,
                    environment,
                    ir_program,
                    errors,
                    lambda_helper,
                );
                let ir_expr = IrExpr::If(ir_cond, ir_true_branch, ir_false_branch);
                return self.add_expr(ir_expr, id, ir_program);
            }
            Expr::Tuple(items) => {
                let ir_items: Vec<IrExprId> = items
                    .iter()
                    .map(|id| {
                        self.process_expr(
                            *id,
                            program,
                            module,
                            environment,
                            ir_program,
                            errors,
                            lambda_helper,
                        )
                    })
                    .collect();
                let ir_expr = IrExpr::Tuple(ir_items);
                return self.add_expr(ir_expr, id, ir_program);
            }
            Expr::Path(path) => {
                match self.resolve_item_path(path, module, environment, lambda_helper) {
                    PathResolveResult::FunctionRef(n) => {
                        let ir_expr = IrExpr::StaticFunctionCall(n, vec![]);
                        return self.add_expr(ir_expr, id, ir_program);
                    }
                    PathResolveResult::VariableRef(named_ref) => {
                        return self.process_named_ref(named_ref, id, ir_program);
                    }
                    PathResolveResult::Unknown(n) => {
                        let err = ResolverError::UnknownFunction(n, id);
                        errors.push(err);
                        let ir_expr = IrExpr::Tuple(vec![]);
                        return self.add_expr(ir_expr, id, ir_program);
                    }
                    PathResolveResult::Ambiguous => {
                        let err = ResolverError::AmbiguousName(path.get(), id);
                        errors.push(err);
                        let ir_expr = IrExpr::Tuple(vec![]);
                        return self.add_expr(ir_expr, id, ir_program);
                    }
                }
            }
            Expr::IntegerLiteral(v) => {
                let ir_expr = IrExpr::IntegerLiteral(v.clone());
                return self.add_expr(ir_expr, id, ir_program);
            }
            Expr::FloatLiteral(v) => {
                let ir_expr = IrExpr::FloatLiteral(v.clone());
                return self.add_expr(ir_expr, id, ir_program);
            }
            Expr::BoolLiteral(v) => {
                let ir_expr = IrExpr::BoolLiteral(v.clone());
                return self.add_expr(ir_expr, id, ir_program);
            }
            Expr::StringLiteral(v) => {
                let ir_expr = IrExpr::StringLiteral(v.clone());
                return self.add_expr(ir_expr, id, ir_program);
            }
            Expr::Do(items) => {
                let ir_items: Vec<IrExprId> = items
                    .iter()
                    .map(|id| {
                        self.process_expr(
                            *id,
                            program,
                            module,
                            environment,
                            ir_program,
                            errors,
                            lambda_helper,
                        )
                    })
                    .collect();
                let ir_expr = IrExpr::Do(ir_items);
                return self.add_expr(ir_expr, id, ir_program);
            }
            Expr::Bind(name, expr_id) => {
                let ir_expr_id = self.process_expr(
                    *expr_id,
                    program,
                    module,
                    environment,
                    ir_program,
                    errors,
                    lambda_helper,
                );
                environment.add_expr_value(name.clone(), ir_expr_id);
                let ir_expr = IrExpr::Bind(name.clone(), ir_expr_id);
                return self.add_expr(ir_expr, id, ir_program);
            }
        }
    }

    pub fn resolve(&mut self, program: &'a Program) -> Result<IrProgram, Error> {
        let mut errors = Vec::new();

        let mut modules = BTreeMap::new();

        let mut ir_program = IrProgram::new();

        for ast_module in program.modules.values() {
            self.register_module(ast_module, &mut modules, &mut ir_program);
        }

        self.process_module_and_export_conflicts(modules)?;
        let mut imported_not_found_modules = Vec::new();

        for (_, ast_module) in &program.modules {
            let mut imported_symbols = ImportStore::new();

            let mut explicit_prelude_import = false;

            for (import_id, import) in &ast_module.imports {
                let imported_module_path = import.module_path.get();
                if imported_module_path == PRELUDE_NAME {
                    explicit_prelude_import = true;
                }
                let source_module = match self.modules.get(&imported_module_path) {
                    Some(module) => module,
                    None => {
                        imported_not_found_modules.push((imported_module_path, import_id.clone()));
                        continue;
                    }
                };
                let (imported_syms, errs) = self.collect_imported_symbols(import, source_module);
                imported_symbols.extend(imported_syms);
                errors.extend(errs);
            }

            if ast_module.name.get() != PRELUDE_NAME && !explicit_prelude_import {
                let source_module = match self.modules.get(PRELUDE_NAME) {
                    Some(module) => module,
                    None => {
                        panic!("Prelude not found");
                    }
                };
                for func in source_module.exported_functions.keys() {
                    imported_symbols.add_imported_function(
                        func.clone(),
                        source_module.name.clone(),
                        PRELUDE_NAME.to_string(),
                        ImportKind::NamespaceOnly,
                    );
                }
            }

            let module = self
                .modules
                .get_mut(&ast_module.name.get())
                .expect("Module not found");
            module.imported_functions.extend(imported_symbols);
        }

        if !imported_not_found_modules.is_empty() {
            let e = ResolverError::ImportedModuleNotFound(imported_not_found_modules);
            errors.push(e);
        }

        for (_, module) in &program.modules {
            for (_, function) in &module.functions {
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
                            ty.full_type_signature_id,
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
                            function.id.clone(),
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
                    let body_id = self.process_expr(
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
