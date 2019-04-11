use crate::error::Error;
use crate::ir::expr::Expr;
use crate::ir::expr::ExprId;
use crate::ir::function::Function;
use crate::ir::function::FunctionId;
use crate::ir::function::FunctionInfo;
use crate::ir::function::NamedFunctionInfo;
use crate::ir::program::Program;
use crate::ir::types::TypeSignature;
use crate::ir::types::TypeSignatureId;
use crate::location_info::item::LocationId;
use crate::typechecker::error::TypecheckError;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use std::collections::BTreeMap;
use std::fmt;

struct FunctionSignatureLocation {
    arg_locations: Vec<LocationId>,
    return_location_id: LocationId,
}

struct FunctionTypeInfo {
    displayed_name: String,
    args: Vec<TypeVariable>,
    signature_args: Vec<TypeVariable>,
    signature_location: Option<FunctionSignatureLocation>,
    result: TypeVariable,
    function_type: TypeVariable,
    body: Option<ExprId>,
}

impl FunctionTypeInfo {
    fn new(
        displayed_name: String,
        args: Vec<TypeVariable>,
        signature_args: Vec<TypeVariable>,
        signature_location: Option<FunctionSignatureLocation>,
        result: TypeVariable,
        function_type: TypeVariable,
        body: Option<ExprId>,
    ) -> FunctionTypeInfo {
        FunctionTypeInfo {
            displayed_name: displayed_name,
            args: args,
            signature_args: signature_args,
            signature_location: signature_location,
            result: result,
            function_type: function_type,
            body: body,
        }
    }
}

impl fmt::Display for FunctionTypeInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut vars = self.args.clone();
        vars.push(self.result);
        let ss: Vec<_> = vars.iter().map(|i| format!("{}", i)).collect();
        write!(f, "{} = {}", self.function_type, ss.join(" -> "))
    }
}

fn report_type_mismatch(
    expected: &TypeVariable,
    found: &TypeVariable,
    type_store: &mut TypeStore,
    expected_id: LocationId,
    found_id: LocationId,
    errors: &mut Vec<TypecheckError>,
) {
    let expected_type = type_store.get_resolved_type_string(&expected);
    let found_type = type_store.get_resolved_type_string(&found);
    let err = TypecheckError::TypeMismatch(expected_id, found_id, expected_type, found_type);
    errors.push(err);
}

fn unify_variables(
    expected: &TypeVariable,
    found: &TypeVariable,
    type_store: &mut TypeStore,
    expected_id: LocationId,
    found_id: LocationId,
    errors: &mut Vec<TypecheckError>,
) {
    if !type_store.unify(&found, &expected) {
        report_type_mismatch(expected, found, type_store, expected_id, found_id, errors);
    }
}

pub struct Typechecker {
    type_store: TypeStore,
    function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    expression_type_var_map: BTreeMap<ExprId, TypeVariable>,
}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {
            type_store: TypeStore::new(),
            function_type_info_map: BTreeMap::new(),
            expression_type_var_map: BTreeMap::new(),
        }
    }

    fn create_type_var_for_expr(&mut self, expr_id: ExprId) -> TypeVariable {
        let var = self.type_store.get_new_type_var();
        self.expression_type_var_map.insert(expr_id, var);
        var
    }

    fn lookup_type_var_for_expr(&self, expr_id: &ExprId) -> TypeVariable {
        *self
            .expression_type_var_map
            .get(expr_id)
            .expect("Type var for expr not found")
    }

    fn process_function_signature_part(
        &mut self,
        type_signature_ids: &[TypeSignatureId],
        program: &Program,
        arg_map: &mut BTreeMap<usize, TypeVariable>,
        signature_arg_locations: &mut Vec<LocationId>,
    ) -> (TypeVariable, LocationId) {
        if type_signature_ids.len() < 2 {
            return self.process_type_signature(
                &type_signature_ids[0],
                program,
                arg_map,
                signature_arg_locations,
            );
        } else {
            let from = &type_signature_ids[0];
            let location_id = program.get_type_signature_location(from);
            signature_arg_locations.push(location_id);
            let (from, _) =
                self.process_type_signature(&from, program, arg_map, signature_arg_locations);
            let (to, to_location_id) = self.process_function_signature_part(
                &type_signature_ids[1..],
                program,
                arg_map,
                signature_arg_locations,
            );
            let ty = Type::Function(FunctionType::new(from, to));
            return (self.type_store.add_type(ty), to_location_id);
        }
    }

    fn process_type_signature(
        &mut self,
        type_signature_id: &TypeSignatureId,
        program: &Program,
        arg_map: &mut BTreeMap<usize, TypeVariable>,
        signature_arg_locations: &mut Vec<LocationId>,
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
                        self.process_type_signature(i, program, arg_map, signature_arg_locations)
                    })
                    .map(|(i, _)| i)
                    .collect();
                let ty = Type::Tuple(items);
                return (self.type_store.add_type(ty), location_id);
            }
            TypeSignature::Function(items) => {
                return self.process_function_signature_part(
                    &items[..],
                    program,
                    arg_map,
                    signature_arg_locations,
                );
            }
            TypeSignature::TypeArgument(index) => {
                let var = arg_map.entry(*index).or_insert_with(|| {
                    let arg = self.type_store.get_unique_type_arg();
                    let ty = Type::TypeArgument(arg);
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
        arg_count: usize,
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
        );
        println!(
            "Registering named function {} {} with type {}",
            function_id,
            displayed_name,
            self.type_store.get_resolved_type_string(&func_type_var)
        );
        let ty = self.type_store.get_type(&func_type_var);
        let function_type_info = match ty {
            Type::Function(func_type) => {
                let mut signature_vars = Vec::new();
                func_type.get_arg_types(&self.type_store, &mut signature_vars);
                if signature_vars.len() < arg_count {
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        named_info.name.clone(),
                        arg_count,
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

                let arg_vars = signature_vars
                    .iter()
                    .map(|v| self.type_store.clone_type_var(*v))
                    .collect();
                FunctionTypeInfo::new(
                    displayed_name,
                    arg_vars,
                    signature_vars,
                    Some(signature_location),
                    func_type.get_return_type(&self.type_store),
                    func_type_var,
                    body,
                )
            }
            _ => {
                if arg_count > 0 {
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        displayed_name,
                        arg_count,
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
                    vec![],
                    Some(signature_location),
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
        for _ in 0..function.arg_count {
            let arg_var = self.type_store.get_new_type_var();
            args.push(arg_var);
        }
        let result = self.type_store.get_new_type_var();
        let func_type_var = if function.arg_count > 0 {
            let mut vars = args.clone();
            vars.push(result);
            while vars.len() > 1 {
                let from = vars[vars.len() - 2];
                let to = vars[vars.len() - 1];
                let func = Type::Function(FunctionType::new(from, to));
                let var = self.type_store.add_type(func);
                let index = vars.len() - 2;
                vars[index] = var;
                vars.pop();
            }
            vars[0]
        } else {
            result
        };
        let signature_args = args.clone();
        let function_type_info = FunctionTypeInfo::new(
            name,
            args,
            signature_args,
            None,
            result,
            func_type_var,
            Some(body),
        );
        self.function_type_info_map.insert(id, function_type_info);
    }

    fn process_expr_and_create_vars(&mut self, program: &Program) {
        for (expr_id, expr_info) in &program.exprs {
            println!("Processing {} {}", expr_id, expr_info.expr);
            match &expr_info.expr {
                Expr::IntegerLiteral(_) => {
                    let var = self.type_store.add_type(Type::Int);
                    self.expression_type_var_map.insert(*expr_id, var);
                }
                Expr::StringLiteral(_) => {
                    let var = self.type_store.add_type(Type::String);
                    self.expression_type_var_map.insert(*expr_id, var);
                }
                _ => {
                    self.create_type_var_for_expr(*expr_id);
                }
            }
        }
    }

    fn check_constraints(&mut self, program: &Program, errors: &mut Vec<TypecheckError>) {
        let mut primary_modified = true;
        while primary_modified {
            primary_modified = false;
            for (expr_id, expr_info) in &program.exprs {
                println!("Checking {} {}", expr_id, expr_info.expr);
                match &expr_info.expr {
                    Expr::IntegerLiteral(_) => {}
                    Expr::StringLiteral(_) => {}
                    Expr::StaticFunctionCall(function_id, args) => {
                        let target_function_type_info = self
                            .function_type_info_map
                            .get(function_id)
                            .expect("Function type info not found");
                    }
                    Expr::ArgRef(arg_ref) => {
                        let function_type_info = self
                            .function_type_info_map
                            .get(&arg_ref.id)
                            .expect("Function type info not found");
                        let arg_var = function_type_info.args[arg_ref.index];
                        let expr_var = self.lookup_type_var_for_expr(expr_id);
                        match &function_type_info.signature_location {
                            Some(locations) => {
                                let copied_arg_var = self.type_store.clone_type_var(arg_var);
                                let arg_location_id = locations.arg_locations[arg_ref.index];
                                let location_id = program.get_expr_location(expr_id);
                                unify_variables(
                                    &copied_arg_var,
                                    &expr_var,
                                    &mut self.type_store,
                                    location_id,
                                    arg_location_id,
                                    errors,
                                );
                            }
                            None => {
                                if !self.type_store.unify(&arg_var, &expr_var) {
                                    panic!("Non typed argument failed to unify with argref");
                                }
                            }
                        };
                    }
                    _ => {
                        panic!("Unimplemented expr {}", expr_info.expr);
                    }
                }
            }
        }

        for (_, function_type_info) in &self.function_type_info_map {
            let body = if let Some(body) = function_type_info.body {
                body
            } else {
                continue;
            };
            let location_id = program.get_expr_location(&body);
            let body_var = self.lookup_type_var_for_expr(&body);
            if let Some(locations) = &function_type_info.signature_location {
                let copied_result = self.type_store.clone_type_var(function_type_info.result);
                unify_variables(
                    &copied_result,
                    &body_var,
                    &mut self.type_store,
                    locations.return_location_id,
                    location_id,
                    errors,
                );
                if self.type_store.modified() {
                    report_type_mismatch(
                        &function_type_info.result,
                        &body_var,
                        &mut self.type_store,
                        locations.return_location_id,
                        location_id,
                        errors,
                    );
                }
                for (index, arg) in function_type_info.args.iter().enumerate() {
                    let signature_arg = &function_type_info.signature_args[index];
                    let signature_location = locations.arg_locations[index];
                    let copied_signature_arg = self.type_store.clone_type_var(*signature_arg);
                    unify_variables(
                        &copied_signature_arg,
                        &arg,
                        &mut self.type_store,
                        signature_location,
                        signature_location,
                        errors,
                    );
                    if self.type_store.modified() {
                        report_type_mismatch(
                            &signature_arg,
                            &arg,
                            &mut self.type_store,
                            signature_location,
                            signature_location,
                            errors,
                        );
                    }
                }
            } else {
                unify_variables(
                    &function_type_info.result,
                    &body_var,
                    &mut self.type_store,
                    location_id,
                    location_id,
                    errors,
                );
            }
        }
    }

    pub fn check(&mut self, program: &Program) -> Result<(), Error> {
        let mut errors = Vec::new();
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
                            function.arg_count,
                            type_signature,
                            *id,
                            program,
                            &mut errors,
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

        /*
        for (id, info) in &self.function_type_info_map {
            println!("{}: {}", id, info);
        }
        */

        self.process_expr_and_create_vars(program);

        self.check_constraints(program, &mut errors);

        //self.type_store.dump();

        for (id, info) in &self.function_type_info_map {
            println!(
                "{}/{}: {}",
                id,
                info.displayed_name,
                self.type_store
                    .get_resolved_type_string(&info.function_type)
            );
        }

        for (expr_id, _) in &program.exprs {
            let var = self.lookup_type_var_for_expr(expr_id);
            println!(
                "Expr: {}: {}",
                expr_id,
                self.type_store.get_resolved_type_string(&var)
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::typecheck_err(errors))
        }
    }
}
