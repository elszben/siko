use crate::error::Error;
use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::function::FunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::program::Program;
use crate::ir::types::TypeSignature;
use crate::ir::types::TypeSignatureId;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use crate::util::format_list;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

struct FunctionDependencyInfo {
    function_deps: BTreeSet<FunctionId>,
}

impl FunctionDependencyInfo {
    fn new() -> FunctionDependencyInfo {
        FunctionDependencyInfo {
            function_deps: BTreeSet::new(),
        }
    }
}

struct FunctionInfoCollector<'a> {
    function_type_info: &'a mut FunctionDependencyInfo,
}

impl<'a> FunctionInfoCollector<'a> {
    fn new(function_type_info: &'a mut FunctionDependencyInfo) -> FunctionInfoCollector<'a> {
        FunctionInfoCollector {
            function_type_info: function_type_info,
        }
    }
}

impl<'a> Collector for FunctionInfoCollector<'a> {
    fn process(&mut self, program: &Program, expr: &Expr, id: ExprId) {
        match expr {
            Expr::StaticFunctionCall(func_id, _) => {
                self.function_type_info.function_deps.insert(*func_id);
            }
            Expr::LambdaFunction(func_id, _) => {
                self.function_type_info.function_deps.insert(*func_id);
            }
            _ => {}
        }
    }
}

struct TypeProcessor<'a> {
    type_store: &'a mut TypeStore,
    function_type_map: &'a BTreeMap<FunctionId, TypeVariable>,
    type_vars: BTreeMap<ExprId, TypeVariable>,
}

impl<'a> TypeProcessor<'a> {
    fn new(
        type_store: &'a mut TypeStore,
        function_type_map: &'a BTreeMap<FunctionId, TypeVariable>,
    ) -> TypeProcessor<'a> {
        TypeProcessor {
            type_store: type_store,
            function_type_map: function_type_map,
            type_vars: BTreeMap::new(),
        }
    }

    fn get_type_var_for_expr(&self, id: &ExprId) -> TypeVariable {
        self.type_vars
            .get(id)
            .expect("Sub expr type var not found")
            .clone()
    }

    fn check_constraints(&mut self, program: &Program, errors: &mut Vec<TypecheckError>) {
        for (id, ty_var) in self.type_vars.clone() {
            let expr = program.get_expr(&id);
            match expr {
                Expr::IntegerLiteral(_) => {}
                Expr::BoolLiteral(_) => {}
                Expr::StringLiteral(_) => {}
                Expr::If(cond, true_branch, false_branch) => {
                    let cond_var = self.get_type_var_for_expr(cond);
                    let true_var = self.get_type_var_for_expr(true_branch);
                    let false_var = self.get_type_var_for_expr(false_branch);
                    let cond_ty = self.type_store.get_type(&cond_var);
                    if cond_ty != Type::Bool {
                        let var = self.type_store.add_var(Type::Bool);
                        if !self.type_store.unify_vars(var, cond_var) {
                            let ast_id = program.get_ast_expr_id(cond);
                            let cond_ty = self.type_store.get_resolved_type(&cond_var);
                            let bool_ty = format!("{}", Type::Bool);
                            let cond_ty = format!("{}", cond_ty);
                            let err = TypecheckError::TypeMismatch(*ast_id, bool_ty, cond_ty);
                            errors.push(err);
                        }
                    }
                    if !self.type_store.unify_vars(true_var, false_var) {
                        let ast_id = program.get_ast_expr_id(&false_branch);
                        let true_type = self.type_store.get_resolved_type(&true_var);
                        let false_type = self.type_store.get_resolved_type(&false_var);
                        let true_type = format!("{}", true_type);
                        let false_type = format!("{}", false_type);
                        let err = TypecheckError::TypeMismatch(*ast_id, true_type, false_type);
                        errors.push(err);
                    }
                }
                Expr::StaticFunctionCall(function_id, args) => {
                    let target_func_type_var = self
                        .function_type_map
                        .get(function_id)
                        .expect("Function type not found");
                    let ty = self.type_store.get_type(target_func_type_var);
                    match ty {
                        Type::Function(function_type) => {
                            let mut arg_map = BTreeMap::new();
                            let types: Vec<_> = function_type
                                .types
                                .iter()
                                .map(|ty| self.type_store.clone_type(ty, &mut arg_map))
                                .collect();
                            if args.len() > types.len() - 1 {
                                let f = program.get_function(function_id);
                                let name = format!("{}", f.info);
                                let ast_id = program.get_ast_expr_id(&id);
                                let err = TypecheckError::TooManyArguments(
                                    *ast_id,
                                    name,
                                    types.len() - 1,
                                    args.len(),
                                );
                                errors.push(err);
                            } else {
                                let mut mismatch = false;
                                for (index, arg) in args.iter().enumerate() {
                                    let arg_var = self.get_type_var_for_expr(arg);
                                    let type_var = types[index].get_inner_type_var();
                                    if !self.type_store.unify_vars(arg_var, type_var) {
                                        mismatch = true;
                                        break;
                                    }
                                }
                                if mismatch {
                                    let ast_id = program.get_ast_expr_id(&id);
                                    let mut arg_types = Vec::new();
                                    for arg in args {
                                        let arg_var = self.get_type_var_for_expr(arg);
                                        let ty = self.type_store.get_resolved_type(&arg_var);
                                        arg_types.push(format!("{}", ty));
                                    }
                                    let arg_types = format_list(&arg_types[..]);
                                    let func_type = function_type.as_string(self.type_store);
                                    let err = TypecheckError::FunctionArgumentMismatch(
                                        *ast_id, arg_types, func_type,
                                    );
                                    errors.push(err);
                                }
                            }
                        }
                        _ => {
                            if !args.is_empty() {
                                let f = program.get_function(function_id);
                                let name = format!("{}", f.info);
                                let ast_id = program.get_ast_expr_id(&id);
                                let err =
                                    TypecheckError::TooManyArguments(*ast_id, name, 0, args.len());
                                errors.push(err);
                            } else {
                                let call_var = self.get_type_var_for_expr(&id);
                                if !self.type_store.unify_vars(call_var, *target_func_type_var) {
                                    let ast_id = program.get_ast_expr_id(&id);
                                    let call_type = self.type_store.get_resolved_type(&call_var);
                                    let func_type =
                                        self.type_store.get_resolved_type(target_func_type_var);
                                    let call_type = format!("{}", call_type);
                                    let func_type = format!("{}", func_type);
                                    let err =
                                        TypecheckError::TypeMismatch(*ast_id, call_type, func_type);
                                    errors.push(err);
                                }
                            }
                        }
                    }
                }
                _ => panic!("Check of expr {} is not implemented", expr),
            }
        }
    }

    fn dump_types(&self, program: &Program) {
        for (id, var) in &self.type_vars {
            let expr = program.get_expr(id);
            let ty = self.type_store.get_resolved_type(var);
            println!("{} {} {}", id, expr, ty);
        }
    }
}

impl<'a> Collector for TypeProcessor<'a> {
    fn process(&mut self, program: &Program, expr: &Expr, id: ExprId) {
        match expr {
            Expr::IntegerLiteral(_) => {
                let ty = Type::Int;
                let var = self.type_store.add_var(ty);
                self.type_vars.insert(id, var);
            }
            Expr::BoolLiteral(_) => {
                let ty = Type::Bool;
                let var = self.type_store.add_var(ty);
                self.type_vars.insert(id, var);
            }
            Expr::StringLiteral(_) => {
                let ty = Type::String;
                let var = self.type_store.add_var(ty);
                self.type_vars.insert(id, var);
            }
            Expr::If(_, true_branch, _) => {
                let true_var = self.get_type_var_for_expr(true_branch);
                self.type_vars.insert(id, true_var);
            }
            Expr::StaticFunctionCall(_, _) => {
                let ty = Type::TypeArgument(self.type_store.get_unique_type_arg());
                let result_var = self.type_store.add_var(ty);
                self.type_vars.insert(id, result_var);
            }
            _ => panic!("Type processing of expr {} is not implemented", expr),
        }
    }
}

trait Collector {
    fn process(&mut self, program: &Program, expr: &Expr, id: ExprId) {}
}

fn walker(program: &Program, id: &ExprId, collector: &mut Collector) {
    let expr = program.get_expr(id);

    match expr {
        Expr::StaticFunctionCall(_, args) => {
            for arg in args {
                walker(program, arg, collector);
            }
        }
        Expr::LambdaFunction(_, args) => {
            for arg in args {
                walker(program, arg, collector);
            }
        }
        Expr::DynamicFunctionCall(id, args) => {
            walker(program, id, collector);
            for arg in args {
                walker(program, arg, collector);
            }
        }
        Expr::If(cond, true_branch, false_branch) => {
            walker(program, cond, collector);
            walker(program, true_branch, collector);
            walker(program, false_branch, collector);
        }
        Expr::Tuple(items) => {
            for item in items {
                walker(program, item, collector)
            }
        }
        Expr::IntegerLiteral(_) => {}
        Expr::FloatLiteral(_) => {}
        Expr::BoolLiteral(_) => {}
        Expr::StringLiteral(_) => {}
        Expr::Do(items) => {
            for item in items {
                walker(program, item, collector)
            }
        }
        Expr::Bind(_, expr) => walker(program, expr, collector),
        Expr::ArgRef(_) => {}
        Expr::ExprValue(_) => {}
    }
    collector.process(program, &expr, *id);
}

pub struct Typechecker {
    function_info_map: BTreeMap<FunctionId, FunctionDependencyInfo>,
    function_type_map: BTreeMap<FunctionId, TypeVariable>,
    type_store: TypeStore,
}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {
            function_info_map: BTreeMap::new(),
            function_type_map: BTreeMap::new(),
            type_store: TypeStore::new(),
        }
    }

    fn check_untyped_function(
        &mut self,
        id: FunctionId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
    ) {
        let function = program.get_function(&id);
        println!("Checking untyped {},{}", id, function.info);
        let body = function.info.body();
        let mut type_processor = TypeProcessor::new(&mut self.type_store, &self.function_type_map);
        walker(program, &body, &mut type_processor);
        type_processor.check_constraints(program, errors);
        type_processor.dump_types(program);
    }

    fn check_function_deps(
        &self,
        mut untyped_functions: BTreeSet<FunctionId>,
        errors: &mut Vec<TypecheckError>,
    ) -> Vec<FunctionId> {
        let mut untyped_check_order = Vec::new();

        while !untyped_functions.is_empty() {
            let mut processed = Vec::new();
            for id in &untyped_functions {
                let info = self
                    .function_info_map
                    .get(id)
                    .expect("Function info not found");
                let mut dep_is_untyped = false;
                for dep in &info.function_deps {
                    if untyped_functions.contains(dep) {
                        dep_is_untyped = true;
                        break;
                    }
                }
                if dep_is_untyped {
                    continue;
                } else {
                    untyped_check_order.push(*id);
                    processed.push(*id);
                }
            }
            if processed.is_empty() {
                let err = TypecheckError::FunctionTypeDependencyLoop;
                errors.push(err);
                break;
            } else {
                for id in processed {
                    untyped_functions.remove(&id);
                }
            }
        }
        untyped_check_order
    }

    fn process_type_signature(
        &mut self,
        type_signature_id: &TypeSignatureId,
        program: &Program,
        arg_map: &mut BTreeMap<usize, usize>,
    ) -> TypeVariable {
        let type_signature = program.get_type_signature(type_signature_id);
        match type_signature {
            TypeSignature::Bool => {
                let ty = Type::Bool;
                return self.type_store.add_var(ty);
            }
            TypeSignature::Int => {
                let ty = Type::Int;
                return self.type_store.add_var(ty);
            }
            TypeSignature::String => {
                let ty = Type::String;
                return self.type_store.add_var(ty);
            }
            TypeSignature::Nothing => {
                let ty = Type::Nothing;
                return self.type_store.add_var(ty);
            }
            TypeSignature::Tuple(items) => {
                let items: Vec<_> = items
                    .iter()
                    .map(|i| Type::TypeVar(self.process_type_signature(i, program, arg_map)))
                    .collect();
                let ty = Type::Tuple(items);
                return self.type_store.add_var(ty);
            }
            TypeSignature::Function(items) => {
                let items: Vec<_> = items
                    .iter()
                    .map(|i| Type::TypeVar(self.process_type_signature(i, program, arg_map)))
                    .collect();
                let ty = Type::Function(FunctionType::new(items));
                return self.type_store.add_var(ty);
            }
            TypeSignature::TypeArgument(index) => {
                let arg = arg_map
                    .entry(*index)
                    .or_insert_with(|| self.type_store.get_unique_type_arg());
                let ty = Type::TypeArgument(*arg);
                return self.type_store.add_var(ty);
            }
        }
    }

    fn add_type_signature(
        &mut self,
        type_signature_id: TypeSignatureId,
        function_id: FunctionId,
        program: &Program,
    ) {
        let mut arg_map = BTreeMap::new();
        let var = self.process_type_signature(&type_signature_id, program, &mut arg_map);
        println!(
            "Registering function {} with type {}",
            function_id,
            self.type_store.get_resolved_type(&var)
        );
        self.function_type_map.insert(function_id, var);
    }

    pub fn check(&mut self, program: &Program) -> Result<(), Error> {
        let mut errors = Vec::new();
        let mut untyped_functions = BTreeSet::new();
        let mut typed_functions = BTreeSet::new();
        for (id, function) in &program.functions {
            let mut function_info = FunctionDependencyInfo::new();
            let mut function_info_collector = FunctionInfoCollector::new(&mut function_info);
            match &function.info {
                FunctionInfo::Lambda(e) => {
                    walker(program, &e, &mut function_info_collector);
                    untyped_functions.insert(*id);
                }
                FunctionInfo::NamedFunction(i) => {
                    let untyped = match i.type_signature {
                        Some(type_signature) => {
                            self.add_type_signature(type_signature, *id, program);
                            false
                        }
                        None => true,
                    };
                    if untyped {
                        untyped_functions.insert(*id);
                    }
                    if let Some(body) = i.body {
                        walker(program, &body, &mut function_info_collector);
                        if !untyped {
                            typed_functions.insert(*id);
                        }
                    } else {
                        if untyped {
                            let err = TypecheckError::UntypedExternFunction(
                                i.name.clone(),
                                i.ast_function_id,
                            );
                            errors.push(err)
                        }
                    }
                }
            }
            self.function_info_map.insert(*id, function_info);
        }

        let untyped_check_order = self.check_function_deps(untyped_functions, &mut errors);

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        for function_id in untyped_check_order {
            self.check_untyped_function(function_id, program, &mut errors);
        }

        for function_id in typed_functions {
            println!("Checking typed {}", function_id);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::typecheck_err(errors))
        }
    }
}
