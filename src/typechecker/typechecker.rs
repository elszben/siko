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
use crate::typechecker::error::TypecheckError;
use crate::typechecker::function_type::FunctionType;
use crate::typechecker::type_store::TypeStore;
use crate::typechecker::type_variable::TypeVariable;
use crate::typechecker::types::Type;
use std::collections::BTreeMap;
use std::fmt;

struct FunctionTypeInfo {
    args: Vec<TypeVariable>,
    result: TypeVariable,
    function_type: TypeVariable,
    typed: bool,
    body: ExprId,
}

impl FunctionTypeInfo {
    fn new(
        args: Vec<TypeVariable>,
        result: TypeVariable,
        function_type: TypeVariable,
        typed: bool,
        body: ExprId,
    ) -> FunctionTypeInfo {
        FunctionTypeInfo {
            args: args,
            result: result,
            function_type: function_type,
            typed: typed,
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

struct FunctionCallInfo {
    called_function_type: TypeVariable,
}

fn unify_variables(
    expected: &TypeVariable,
    found: &TypeVariable,
    type_store: &mut TypeStore,
    program: &Program,
    id: ExprId,
    unified_variables: &mut bool,
    errors: &mut Vec<TypecheckError>,
) {
    if !type_store.unify(&expected, &found, unified_variables) {
        let location_id = program.get_expr_location(&id);
        let expected_type = type_store.get_resolved_type_string(&expected);
        let found_type = type_store.get_resolved_type_string(&found);
        let err = TypecheckError::TypeMismatch(location_id, expected_type, found_type);
        errors.push(err);
    }
}

pub struct Typechecker {
    type_store: TypeStore,
    function_type_info_map: BTreeMap<FunctionId, FunctionTypeInfo>,
    expression_type_var_map: BTreeMap<ExprId, TypeVariable>,
    function_call_info_map: BTreeMap<ExprId, FunctionCallInfo>,
}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {
            type_store: TypeStore::new(),
            function_type_info_map: BTreeMap::new(),
            expression_type_var_map: BTreeMap::new(),
            function_call_info_map: BTreeMap::new(),
        }
    }

    fn get_new_type_var(&mut self) -> TypeVariable {
        let ty = self.type_store.get_unique_type_arg_type();
        let var = self.type_store.add_var(ty);
        var
    }

    fn get_type_var_for_expr(&mut self, expr_id: ExprId) -> TypeVariable {
        match self.expression_type_var_map.get(&expr_id) {
            Some(var) => *var,
            None => {
                let var = self.get_new_type_var();
                self.expression_type_var_map.insert(expr_id, var);
                var
            }
        }
    }

    fn lookup_type_var_for_expr(&self, expr_id: ExprId) -> TypeVariable {
        *self
            .expression_type_var_map
            .get(&expr_id)
            .expect("Type var for expr not found")
    }

    fn process_function_signature_part(
        &mut self,
        type_signature_ids: &[TypeSignatureId],
        program: &Program,
        arg_map: &mut BTreeMap<usize, TypeVariable>,
    ) -> TypeVariable {
        if type_signature_ids.len() < 2 {
            return self.process_type_signature(&type_signature_ids[0], program, arg_map);
        } else {
            let from = &type_signature_ids[0];
            let from = self.process_type_signature(&from, program, arg_map);
            let to =
                self.process_function_signature_part(&type_signature_ids[1..], program, arg_map);
            let ty = Type::Function(FunctionType::new(from, to));
            return self.type_store.add_var(ty);
        }
    }

    fn process_type_signature(
        &mut self,
        type_signature_id: &TypeSignatureId,
        program: &Program,
        arg_map: &mut BTreeMap<usize, TypeVariable>,
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
                    .map(|i| self.process_type_signature(i, program, arg_map))
                    .collect();
                let ty = Type::Tuple(items);
                return self.type_store.add_var(ty);
            }
            TypeSignature::Function(items) => {
                return self.process_function_signature_part(&items[..], program, arg_map);
            }
            TypeSignature::TypeArgument(index) => {
                let var = arg_map.entry(*index).or_insert_with(|| {
                    let arg = self.type_store.get_unique_type_arg();
                    let ty = Type::TypeArgument {
                        index: arg,
                        user_defined: true,
                    };
                    self.type_store.add_var(ty)
                });
                *var
            }
            TypeSignature::Named(..) => {
                let ty = Type::Nothing;
                return self.type_store.add_var(ty);
            }
            TypeSignature::Variant(..) => {
                let ty = Type::Nothing;
                return self.type_store.add_var(ty);
            }
        }
    }

    fn register_typed_function(
        &mut self,
        named_info: &NamedFunctionInfo,
        arg_count: usize,
        type_signature_id: TypeSignatureId,
        function_id: FunctionId,
        program: &Program,
        errors: &mut Vec<TypecheckError>,
        body: ExprId,
    ) {
        let mut arg_map = BTreeMap::new();
        let func_type_var = self.process_type_signature(&type_signature_id, program, &mut arg_map);
        println!(
            "Registering function {} with type {}",
            function_id,
            self.type_store.get_resolved_type_string(&func_type_var)
        );
        let ty = self.type_store.get_type(&func_type_var);
        let function_type_info = match ty {
            Type::Function(func_type) => {
                let mut arg_vars = Vec::new();
                func_type.get_arg_types(&self.type_store, &mut arg_vars);
                if arg_vars.len() < arg_count {
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        named_info.name.clone(),
                        arg_count,
                        arg_vars.len(),
                        program.get_type_signature_location(&type_signature_id),
                    );
                    errors.push(err);
                    return;
                }
                FunctionTypeInfo::new(
                    arg_vars,
                    func_type.get_return_type(&self.type_store),
                    func_type_var,
                    true,
                    body,
                )
            }
            _ => {
                if arg_count > 0 {
                    let err = TypecheckError::FunctionArgAndSignatureMismatch(
                        named_info.name.clone(),
                        arg_count,
                        0,
                        program.get_type_signature_location(&type_signature_id),
                    );
                    errors.push(err);
                    return;
                }
                FunctionTypeInfo::new(vec![], func_type_var, func_type_var, true, body)
            }
        };
        self.function_type_info_map
            .insert(function_id, function_type_info);
    }

    fn register_untyped_function(&mut self, id: FunctionId, function: &Function, body: ExprId) {
        let mut args = Vec::new();
        for _ in 0..function.arg_count {
            let arg_var = self.get_new_type_var();
            args.push(arg_var);
        }
        let result = self.get_new_type_var();
        let func_type_var = if function.arg_count > 0 {
            let mut vars = args.clone();
            vars.push(result);
            while vars.len() > 1 {
                let from = vars[vars.len() - 2];
                let to = vars[vars.len() - 1];
                let func = Type::Function(FunctionType::new(from, to));
                let var = self.type_store.add_var(func);
                let index = vars.len() - 2;
                vars[index] = var;
                vars.pop();
            }
            vars[0]
        } else {
            result
        };
        let function_type_info = FunctionTypeInfo::new(args, result, func_type_var, false, body);
        self.function_type_info_map.insert(id, function_type_info);
    }

    fn process_expr_and_create_vars(&mut self, program: &Program) {
        for (expr_id, expr_info) in &program.exprs {
            println!("Processing {} {}", expr_id, expr_info.expr);
            self.get_type_var_for_expr(*expr_id);
            match &expr_info.expr {
                Expr::StaticFunctionCall(_, args) => {
                    let called_function_type = if args.is_empty() {
                        self.get_new_type_var()
                    } else {
                        let from = self.get_new_type_var();
                        let mut to = self.get_new_type_var();
                        for _ in 0..args.len() {
                            let func_ty = Type::Function(FunctionType::new(from, to));
                            let var = self.type_store.add_var(func_ty);
                            to = var;
                        }
                        to
                    };
                    self.function_call_info_map.insert(
                        *expr_id,
                        FunctionCallInfo {
                            called_function_type: called_function_type,
                        },
                    );
                }
                _ => {}
            }
        }
    }

    fn check_constraints(&mut self, program: &Program, errors: &mut Vec<TypecheckError>) {
        let mut unified_variables = false;

        for (expr_id, expr_info) in &program.exprs {
            println!("Checking {} {}", expr_id, expr_info.expr);
            match &expr_info.expr {
                Expr::IntegerLiteral(_) => {
                    let var = self.get_type_var_for_expr(*expr_id);
                    self.unify_variable_with_type(
                        &var,
                        &Type::Int,
                        program,
                        *expr_id,
                        &mut unified_variables,
                        errors,
                    );
                }
                Expr::StringLiteral(_) => {
                    let var = self.get_type_var_for_expr(*expr_id);
                    self.unify_variable_with_type(
                        &var,
                        &Type::String,
                        program,
                        *expr_id,
                        &mut unified_variables,
                        errors,
                    );
                }
                Expr::StaticFunctionCall(function_id, args) => {}
                _ => {
                    panic!("Unimplemented expr {}", expr_info.expr);
                }
            }
        }

        for (_, function_type_info) in &self.function_type_info_map {
            if !function_type_info.typed {
                let body_var = self.lookup_type_var_for_expr(function_type_info.body);
                unify_variables(
                    &body_var,
                    &function_type_info.result,
                    &mut self.type_store,
                    program,
                    function_type_info.body,
                    &mut unified_variables,
                    errors,
                );
            }
        }
    }

    fn unify_variable_with_type(
        &mut self,
        var: &TypeVariable,
        ty: &Type,
        program: &Program,
        id: ExprId,
        unified_variables: &mut bool,
        errors: &mut Vec<TypecheckError>,
    ) {
        if !self
            .type_store
            .unify_variable_with_type(var, ty, unified_variables)
        {
            let location_id = program.get_expr_location(&id);
            let found_type = self.type_store.get_resolved_type_string(var);
            let expected_type = ty.as_string(&self.type_store, false);
            let err = TypecheckError::TypeMismatch(location_id, expected_type, found_type);
            errors.push(err);
        }
    }

    pub fn check(&mut self, program: &Program) -> Result<(), Error> {
        let mut errors = Vec::new();
        for (id, function) in &program.functions {
            match &function.info {
                FunctionInfo::RecordConstructor(_) => {}
                FunctionInfo::VariantConstructor(_) => {}
                FunctionInfo::Lambda(_) => {}
                FunctionInfo::NamedFunction(i) => match i.type_signature {
                    Some(type_signature) => {
                        if let Some(body) = i.body {
                            self.register_typed_function(
                                i,
                                function.arg_count,
                                type_signature,
                                *id,
                                program,
                                &mut errors,
                                body,
                            );
                        }
                    }
                    None => match i.body {
                        Some(body) => {
                            self.register_untyped_function(*id, function, body);
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

        for (id, info) in &self.function_type_info_map {
            println!("{}: {}", id, info);
        }

        self.process_expr_and_create_vars(program);

        self.check_constraints(program, &mut errors);

        self.type_store.dump();

        for (id, info) in &self.function_type_info_map {
            println!(
                "{}: {}",
                id,
                self.type_store
                    .get_resolved_type_string(&info.function_type)
            );
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::typecheck_err(errors))
        }
    }
}
