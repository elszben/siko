use crate::common::AdtTypeInfo;
use crate::common::ClassMemberTypeInfo;
use crate::common::DeriveInfo;
use crate::common::FunctionTypeInfo;
use crate::common::FunctionTypeInfoStore;
use crate::common::RecordTypeInfo;
use crate::common::VariantTypeInfo;
use crate::dependency_processor::DependencyGroup;
use crate::error::Error;
use crate::error::TypecheckError;
use crate::function_dep_processor::FunctionDependencyProcessor;
use crate::instance_resolver::InstanceResolver;
use crate::substitution::Substitution;
use crate::type_var_generator::TypeVarGenerator;
use crate::types::BaseType;
use crate::types::ResolverContext;
use crate::types::Type;
use crate::unifier::Unifier;
use crate::util::create_general_function_type;
use siko_ir::class::ClassId;
use siko_ir::class::ClassMemberId;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::Function;
use siko_ir::function::FunctionId;
use siko_ir::function::FunctionInfo;
use siko_ir::function::NamedFunctionKind;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_ir::types::TypeDef;
use siko_ir::types::TypeDefId;
use siko_ir::types::TypeSignature;
use siko_ir::types::TypeSignatureId;
use siko_ir::walker::walk_expr;
use siko_ir::walker::Visitor;
use siko_location_info::item::LocationId;
use siko_util::format_list;
use siko_util::RcCounter;
use std::collections::BTreeMap;

fn process_type_signature(
    type_signature_id: TypeSignatureId,
    program: &Program,
    type_var_generator: &mut TypeVarGenerator,
) -> Type {
    let type_signature = &program.type_signatures.get(&type_signature_id).item;
    match type_signature {
        TypeSignature::Function(from, to) => {
            let from_ty = process_type_signature(*from, program, type_var_generator);
            let to_ty = process_type_signature(*to, program, type_var_generator);
            Type::Function(Box::new(from_ty), Box::new(to_ty))
        }
        TypeSignature::Named(name, id, items) => {
            let items: Vec<_> = items
                .iter()
                .map(|item| process_type_signature(*item, program, type_var_generator))
                .collect();
            Type::Named(name.clone(), *id, items)
        }
        TypeSignature::Tuple(items) => {
            let items: Vec<_> = items
                .iter()
                .map(|item| process_type_signature(*item, program, type_var_generator))
                .collect();
            Type::Tuple(items)
        }
        TypeSignature::TypeArgument(index, name, constraints) => {
            let mut constraints = constraints.clone();
            // unifier assumes that the constraints are sorted!
            constraints.sort();
            Type::FixedTypeArg(name.clone(), *index, constraints)
        }
        TypeSignature::Variant(..) => panic!("Variant should not appear here"),
        TypeSignature::Wildcard => type_var_generator.get_new_type_var(),
    }
}

fn check_instance(
    class_id: ClassId,
    ty: &Type,
    location_id: LocationId,
    instance_resolver: &InstanceResolver,
    substitutions: &mut Vec<(Substitution, LocationId)>,
    mut type_var_generator: TypeVarGenerator,
) -> bool {
    //println!("Checking instance {} {}", class_id, ty);
    if let Type::Var(index, constraints) = ty {
        if constraints.contains(&class_id) {
            return true;
        } else {
            let mut new_constraints = constraints.clone();
            new_constraints.push(class_id);
            let new_type = Type::Var(type_var_generator.get_new_index(), new_constraints);
            let mut sub = Substitution::empty();
            sub.add(*index, &new_type).expect("sub add failed");
            substitutions.push((sub, location_id));
            return true;
        }
    }
    if let Some(sub) = instance_resolver.has_instance(&ty, class_id, type_var_generator.clone()) {
        let constraints = sub.get_constraints();
        substitutions.push((sub, location_id));
        for constraint in constraints {
            if constraint.ty.get_base_type() == BaseType::Generic {
                unimplemented!();
            } else {
                if !check_instance(
                    constraint.class_id,
                    &constraint.ty,
                    location_id,
                    instance_resolver,
                    substitutions,
                    type_var_generator.clone(),
                ) {
                    return false;
                }
            }
        }
        return true;
    } else {
        return false;
    }
}

fn process_type_change(
    target_ty: Type,
    source_index: usize,
    instance_resolver: &mut InstanceResolver,
    instance_index: usize,
    errors: &mut Vec<TypecheckError>,
    adt_name: &str,
    class_name: &str,
    location_id: LocationId,
) -> bool {
    let mut instance_changed = false;
    match target_ty {
        Type::Var(_, target_constraints) => {
            let mut instance = instance_resolver
                .get_auto_derived_instance(instance_index)
                .clone();
            let new_instance_ty = match instance.ty.clone() {
                Type::Named(name, id, args) => {
                    let mut new_args = Vec::new();
                    for arg in args {
                        match arg {
                            Type::Var(var_index, mut constraints) => {
                                if var_index == source_index {
                                    for t in &target_constraints {
                                        if constraints.contains(&t) {
                                            continue;
                                        } else {
                                            instance_changed = true;
                                            constraints.push(*t);
                                        }
                                    }
                                }
                                let new_type = Type::Var(var_index, constraints);
                                new_args.push(new_type);
                            }
                            _ => unreachable!(),
                        }
                    }
                    Type::Named(name, id, new_args)
                }
                _ => unreachable!(),
            };
            instance.ty = new_instance_ty;
            instance_resolver.update_auto_derived_instance(instance_index, instance);
        }
        _ => {
            let err = TypecheckError::DeriveFailureInstanceNotGeneric(
                adt_name.to_string(),
                class_name.to_string(),
                location_id,
            );
            errors.push(err);
        }
    }
    instance_changed
}

#[derive(Debug, Eq, PartialEq)]
pub enum ExpressionTypeState {
    ExprType(Type),
    FunctionCall(Type, Type),
}

impl ExpressionTypeState {
    pub fn apply(&mut self, unifier: &Unifier) {
        match self {
            ExpressionTypeState::ExprType(ty) => {
                *ty = unifier.apply(ty);
            }
            ExpressionTypeState::FunctionCall(func_ty, ty) => {
                *func_ty = unifier.apply(func_ty);
                *ty = unifier.apply(ty);
            }
        }
    }
}

pub struct TypeStore {
    expr_types: BTreeMap<ExprId, ExpressionTypeState>,
    pattern_types: BTreeMap<PatternId, Type>,
}

impl TypeStore {
    pub fn new() -> TypeStore {
        TypeStore {
            expr_types: BTreeMap::new(),
            pattern_types: BTreeMap::new(),
        }
    }

    pub fn initialize_expr(&mut self, expr_id: ExprId, ty: Type) {
        let r = self
            .expr_types
            .insert(expr_id, ExpressionTypeState::ExprType(ty));
        assert_eq!(r, None);
    }

    pub fn initialize_expr_with_func(&mut self, expr_id: ExprId, ty: Type, func_ty: Type) {
        let r = self
            .expr_types
            .insert(expr_id, ExpressionTypeState::FunctionCall(func_ty, ty));
        assert_eq!(r, None);
    }

    pub fn initialize_pattern(&mut self, pattern_id: PatternId, ty: Type) {
        self.pattern_types.insert(pattern_id, ty);
    }

    pub fn get_expr_type(&self, expr_id: &ExprId) -> &Type {
        match self.expr_types.get(expr_id).expect("Expr type not found") {
            ExpressionTypeState::ExprType(ty) => ty,
            ExpressionTypeState::FunctionCall(_, ty) => ty,
        }
    }

    pub fn get_func_type_for_expr(&self, expr_id: &ExprId) -> &Type {
        match self.expr_types.get(expr_id).expect("Expr type not found") {
            ExpressionTypeState::ExprType(_) => unreachable!(),
            ExpressionTypeState::FunctionCall(func_type, _) => func_type,
        }
    }

    pub fn get_pattern_type(&self, pattern_id: &PatternId) -> &Type {
        self.pattern_types
            .get(pattern_id)
            .expect("Pattern type not found")
    }

    pub fn apply(&mut self, unifier: &Unifier) {
        for (_, expr_ty) in &mut self.expr_types {
            expr_ty.apply(unifier);
        }
        for (_, pattern_ty) in &mut self.pattern_types {
            *pattern_ty = unifier.apply(pattern_ty);
        }
    }

    pub fn dump(&self, program: &Program) {
        let mut context = ResolverContext::new(program);
        for (id, _) in &self.expr_types {
            let expr_ty = self.get_expr_type(id);
            println!(
                "E: {}: {}",
                id,
                expr_ty.get_resolved_type_string_with_context(&mut context)
            );
        }
        for (id, pattern_ty) in &self.pattern_types {
            println!(
                "P: {}: {}",
                id,
                pattern_ty.get_resolved_type_string_with_context(&mut context)
            );
        }
    }
}

struct TypeStoreInitializer<'a> {
    program: &'a Program,
    group: &'a DependencyGroup<FunctionId>,
    type_store: &'a mut TypeStore,
    function_type_info_store: &'a mut FunctionTypeInfoStore,
    class_member_type_info_map: &'a BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
    adt_type_info_map: &'a BTreeMap<TypeDefId, AdtTypeInfo>,
    type_var_generator: TypeVarGenerator,
}

impl<'a> TypeStoreInitializer<'a> {
    fn new(
        program: &'a Program,
        group: &'a DependencyGroup<FunctionId>,
        type_store: &'a mut TypeStore,
        function_type_info_store: &'a mut FunctionTypeInfoStore,
        class_member_type_info_map: &'a BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
        adt_type_info_map: &'a BTreeMap<TypeDefId, AdtTypeInfo>,
        type_var_generator: TypeVarGenerator,
    ) -> TypeStoreInitializer<'a> {
        TypeStoreInitializer {
            program: program,
            group: group,
            type_store: type_store,
            function_type_info_store: function_type_info_store,
            class_member_type_info_map: class_member_type_info_map,
            type_var_generator: type_var_generator,
            adt_type_info_map: adt_type_info_map,
        }
    }

    fn get_bool_type(&self) -> Type {
        let id = self.program.get_named_type("Data.Bool", "Bool");
        Type::Named("Bool".to_string(), id, Vec::new())
    }
}

impl<'a> Visitor for TypeStoreInitializer<'a> {
    fn get_program(&self) -> &Program {
        &self.program
    }

    fn visit_expr(&mut self, expr_id: ExprId, expr: &Expr) {
        //println!("I {} {}", expr_id, expr);
        match expr {
            Expr::ArgRef(_) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::Bind(_, _) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::BoolLiteral(_) => {
                self.type_store
                    .initialize_expr(expr_id, self.get_bool_type());
            }
            Expr::ClassFunctionCall(class_member_id, args) => {
                let class_member_type_info = self
                    .class_member_type_info_map
                    .get(class_member_id)
                    .expect("Class member type info not found");
                let mut arg_map = BTreeMap::new();
                let function_type = class_member_type_info
                    .ty
                    .duplicate(&mut arg_map, &mut self.type_var_generator)
                    .remove_fixed_types();
                let result_ty = function_type.get_result_type(args.len());
                self.type_store
                    .initialize_expr_with_func(expr_id, result_ty, function_type);
            }
            Expr::DynamicFunctionCall(_, args) => {
                let mut func_args = Vec::new();
                let (function_type, result_ty) = create_general_function_type(
                    &mut func_args,
                    args.len(),
                    &mut self.type_var_generator,
                );
                self.type_store
                    .initialize_expr_with_func(expr_id, result_ty, function_type);
            }
            Expr::Do(_) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::ExprValue(_, _) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::FloatLiteral(_) => {
                let id = self.program.get_named_type("Data.Float", "Float");
                self.type_store
                    .initialize_expr(expr_id, Type::Named("Float".to_string(), id, Vec::new()));
            }
            Expr::If(..) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_expr(expr_id, ty);
            }
            Expr::IntegerLiteral(_) => {
                let id = self.program.get_named_type("Data.Int", "Int");
                self.type_store
                    .initialize_expr(expr_id, Type::Named("Int".to_string(), id, Vec::new()));
            }
            Expr::Tuple(items) => {
                let item_types: Vec<_> = items
                    .iter()
                    .map(|_| self.type_var_generator.get_new_type_var())
                    .collect();
                let tuple_ty = Type::Tuple(item_types);
                self.type_store.initialize_expr(expr_id, tuple_ty);
            }
            Expr::StaticFunctionCall(function_id, args) => {
                let function_type = if self.group.items.contains(function_id) {
                    self.function_type_info_store
                        .get(function_id)
                        .function_type
                        .clone()
                } else {
                    let mut arg_map = BTreeMap::new();
                    self.function_type_info_store
                        .get(function_id)
                        .function_type
                        .duplicate(&mut arg_map, &mut self.type_var_generator)
                        .remove_fixed_types()
                };
                let result_ty = function_type.get_result_type(args.len());
                self.type_store
                    .initialize_expr_with_func(expr_id, result_ty, function_type);
            }
            Expr::StringLiteral(_) => {
                let id = self.program.get_named_type("Data.String", "String");
                self.type_store
                    .initialize_expr(expr_id, Type::Named("String".to_string(), id, Vec::new()));
            }
            _ => panic!("init of {} not yet implemented", expr),
        }
    }

    fn visit_pattern(&mut self, pattern_id: PatternId, pattern: &Pattern) {
        //println!("I {} {:?}", pattern_id, pattern);
        match pattern {
            Pattern::Binding(_) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_pattern(pattern_id, ty);
            }
            Pattern::Tuple(items) => {
                let item_types: Vec<_> = items
                    .iter()
                    .map(|_| self.type_var_generator.get_new_type_var())
                    .collect();
                let tuple_ty = Type::Tuple(item_types);
                self.type_store.initialize_pattern(pattern_id, tuple_ty);
            }
            Pattern::Variant(typedef_id, index, args) => {
                let ty = self.type_var_generator.get_new_type_var();
                self.type_store.initialize_pattern(pattern_id, ty);
            }
            _ => panic!("{:?} NYI", pattern),
        }
    }
}

struct ExpressionChecker<'a> {
    program: &'a Program,
    group: &'a DependencyGroup<FunctionId>,
    type_store: &'a mut TypeStore,
    type_var_generator: TypeVarGenerator,
    function_type_info_store: &'a mut FunctionTypeInfoStore,
    errors: &'a mut Vec<TypecheckError>,
}

impl<'a> ExpressionChecker<'a> {
    fn new(
        program: &'a Program,
        group: &'a DependencyGroup<FunctionId>,
        type_store: &'a mut TypeStore,
        type_var_generator: TypeVarGenerator,
        function_type_info_store: &'a mut FunctionTypeInfoStore,
        errors: &'a mut Vec<TypecheckError>,
    ) -> ExpressionChecker<'a> {
        ExpressionChecker {
            program: program,
            group: group,
            type_store: type_store,
            type_var_generator: type_var_generator,
            function_type_info_store: function_type_info_store,
            errors: errors,
        }
    }

    fn unify(&mut self, ty1: &Type, ty2: &Type, location: LocationId) {
        let mut unifier = Unifier::new(self.type_var_generator.clone());
        if unifier.unify(ty1, ty2).is_err() {
            let ty_str1 = ty1.get_resolved_type_string(self.program);
            let ty_str2 = ty2.get_resolved_type_string(self.program);
            let err = TypecheckError::TypeMismatch(location, ty_str1, ty_str2);
            self.errors.push(err);
        } else {
            let cs = unifier.get_constraints();
            println!("cs{:?}", cs);
            self.type_store.apply(&unifier);
            for id in &self.group.items {
                let info = self.function_type_info_store.get_mut(id);
                info.apply(&unifier);
            }
        }
    }

    fn match_expr_with(&mut self, expr_id: ExprId, ty: &Type) {
        let expr_ty = self.type_store.get_expr_type(&expr_id).clone();
        let location = self.program.exprs.get(&expr_id).location_id;
        self.unify(ty, &expr_ty, location);
    }

    fn match_pattern_with(&mut self, pattern_id: PatternId, ty: &Type) {
        let pattern_ty = self.type_store.get_pattern_type(&pattern_id).clone();
        let location = self.program.patterns.get(&pattern_id).location_id;
        self.unify(ty, &pattern_ty, location);
    }

    fn match_expr_with_pattern(&mut self, expr_id: ExprId, pattern_id: PatternId) {
        let expr_ty = self.type_store.get_expr_type(&expr_id).clone();
        let pattern_ty = self.type_store.get_pattern_type(&pattern_id).clone();
        let location = self.program.patterns.get(&pattern_id).location_id;
        self.unify(&expr_ty, &pattern_ty, location);
    }

    fn match_exprs(&mut self, expr_id1: ExprId, expr_id2: ExprId) {
        let expr_ty1 = self.type_store.get_expr_type(&expr_id1).clone();
        let expr_ty2 = self.type_store.get_expr_type(&expr_id2).clone();
        let location = self.program.exprs.get(&expr_id1).location_id;
        self.unify(&expr_ty1, &expr_ty2, location);
    }

    fn check_function_call(&mut self, expr_id: ExprId, args: &Vec<ExprId>) {
        let func_type = self.type_store.get_func_type_for_expr(&expr_id);
        let arg_count = func_type.get_arg_count();
        if arg_count >= args.len() {
            let mut arg_types = Vec::new();
            func_type.get_args(&mut arg_types);
            for (arg, arg_type) in args.iter().zip(arg_types.iter()) {
                self.match_expr_with(*arg, arg_type);
            }
        } else {
            let mut context = ResolverContext::new(self.program);
            let function_type_string =
                func_type.get_resolved_type_string_with_context(&mut context);
            let arg_type_strings: Vec<_> = args
                .iter()
                .map(|arg| {
                    let ty = self.type_store.get_expr_type(arg);
                    ty.get_resolved_type_string_with_context(&mut context)
                })
                .collect();
            let location = self.program.exprs.get(&expr_id).location_id;
            let arguments = format_list(&arg_type_strings[..]);
            let err =
                TypecheckError::FunctionArgumentMismatch(location, arguments, function_type_string);
            self.errors.push(err);
        }
    }

    fn get_bool_type(&self) -> Type {
        let id = self.program.get_named_type("Data.Bool", "Bool");
        Type::Named("Bool".to_string(), id, Vec::new())
    }
}

impl<'a> Visitor for ExpressionChecker<'a> {
    fn get_program(&self) -> &Program {
        &self.program
    }

    fn visit_expr(&mut self, expr_id: ExprId, expr: &Expr) {
        //self.expr_processor.create_type_var_for_expr(expr_id);
        //println!("C {} {}", expr_id, expr);
        match expr {
            Expr::ArgRef(arg_ref) => {
                let func = self.program.functions.get(&arg_ref.id);
                let index = if arg_ref.captured {
                    arg_ref.index
                } else {
                    func.implicit_arg_count + arg_ref.index
                };
                let func_type_info = self.function_type_info_store.get(&arg_ref.id);
                let arg_ty = func_type_info.args[index].clone();
                self.match_expr_with(expr_id, &arg_ty);
            }
            Expr::Bind(pattern_id, rhs) => {
                self.match_expr_with_pattern(*rhs, *pattern_id);
                let expr_ty = Type::Tuple(Vec::new());
                self.match_expr_with(expr_id, &expr_ty);
            }
            Expr::BoolLiteral(_) => {}
            Expr::ClassFunctionCall(_, args) => {
                self.check_function_call(expr_id, args);
            }
            Expr::DynamicFunctionCall(func_expr_id, args) => {
                let func_type = self.type_store.get_func_type_for_expr(&expr_id).clone();
                self.match_expr_with(*func_expr_id, &func_type);
                self.check_function_call(expr_id, args);
            }
            Expr::Do(items) => {
                let last_expr_id = items[items.len() - 1];
                self.match_exprs(expr_id, last_expr_id);
            }
            Expr::ExprValue(_, pattern_id) => {
                self.match_expr_with_pattern(expr_id, *pattern_id);
            }
            Expr::FloatLiteral(_) => {}
            Expr::If(cond, true_branch, false_branch) => {
                let bool_ty = self.get_bool_type();
                self.match_expr_with(*cond, &bool_ty);
                self.match_exprs(*true_branch, *false_branch);
                self.match_exprs(expr_id, *true_branch);
            }
            Expr::IntegerLiteral(_) => {}
            Expr::Tuple(items) => {
                let item_types: Vec<_> = items
                    .iter()
                    .map(|item| self.type_store.get_expr_type(item).clone())
                    .collect();
                let tuple_ty = Type::Tuple(item_types);
                self.match_expr_with(expr_id, &tuple_ty);
            }
            Expr::StaticFunctionCall(_, args) => {
                self.check_function_call(expr_id, args);
            }
            Expr::StringLiteral(_) => {}
            _ => unimplemented!(),
        }
    }

    fn visit_pattern(&mut self, pattern_id: PatternId, pattern: &Pattern) {
        //println!("C {} {:?}", pattern_id, pattern);
        match pattern {
            Pattern::Binding(_) => {}
            Pattern::Tuple(items) => {
                let item_types: Vec<_> = items
                    .iter()
                    .map(|item| self.type_store.get_pattern_type(item).clone())
                    .collect();
                let tuple_ty = Type::Tuple(item_types);
                self.match_pattern_with(pattern_id, &tuple_ty);
            }
            _ => unimplemented!(),
        }
    }
}

pub struct Typechecker {}

impl Typechecker {
    pub fn new() -> Typechecker {
        Typechecker {}
    }

    fn process_derived_instances_instances(
        &self,
        instance_resolver: &mut InstanceResolver,
        errors: &mut Vec<TypecheckError>,
        program: &Program,
        type_var_generator: &mut TypeVarGenerator,
        adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
        record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
    ) {
        instance_resolver.check_conflicts(errors, program, type_var_generator.clone());

        if !errors.is_empty() {
            return;
        }

        loop {
            let mut instance_changed = false;
            for (id, adt_type_info) in adt_type_info_map {
                let adt = program.typedefs.get(id).get_adt();
                for derive_info in &adt_type_info.derived_classes {
                    let class = program.classes.get(&derive_info.class_id);
                    //println!("Processing derived_class {} for {}", class.name, adt.name);
                    let mut substitutions = Vec::new();
                    for variant_type in &adt_type_info.variant_types {
                        for item_type in &variant_type.item_types {
                            if check_instance(
                                derive_info.class_id,
                                &item_type.0,
                                item_type.1,
                                &instance_resolver,
                                &mut substitutions,
                                type_var_generator.clone(),
                            ) {
                            } else {
                                let err = TypecheckError::DeriveFailureNoInstanceFound(
                                    adt.name.clone(),
                                    class.name.clone(),
                                    item_type.1,
                                );
                                errors.push(err);
                                //println!("{:?} does not implement {}", item_type.1, class.name);
                            }
                        }
                    }
                    for (sub, location_id) in substitutions {
                        for (index, target_ty) in sub.get_changes() {
                            if process_type_change(
                                target_ty.clone(),
                                *index,
                                instance_resolver,
                                derive_info.instance_index,
                                errors,
                                &adt.name,
                                &class.name,
                                location_id,
                            ) {
                                instance_changed = true;
                            }
                        }
                    }
                }
            }

            for (id, record_type_info) in record_type_info_map {
                let record = program.typedefs.get(id).get_record();
                for derive_info in &record_type_info.derived_classes {
                    let class = program.classes.get(&derive_info.class_id);
                    //println!("Processing derived_class {} for {}", class.name, record.name);
                    let mut substitutions = Vec::new();
                    for field_type in &record_type_info.field_types {
                        if check_instance(
                            derive_info.class_id,
                            &field_type.0,
                            field_type.1,
                            &instance_resolver,
                            &mut substitutions,
                            type_var_generator.clone(),
                        ) {
                        } else {
                            let err = TypecheckError::DeriveFailureNoInstanceFound(
                                record.name.clone(),
                                class.name.clone(),
                                field_type.1,
                            );
                            errors.push(err);
                            //println!("{:?} does not implement {}", item_type.1, class.name);
                        }
                    }
                    for (sub, location_id) in substitutions {
                        for (index, target_ty) in sub.get_changes() {
                            if process_type_change(
                                target_ty.clone(),
                                *index,
                                instance_resolver,
                                derive_info.instance_index,
                                errors,
                                &record.name,
                                &class.name,
                                location_id,
                            ) {
                                instance_changed = true;
                            }
                        }
                    }
                }
            }

            if !instance_changed {
                break;
            }

            if !errors.is_empty() {
                break;
            }
        }
    }

    fn process_data_types(
        &self,
        instance_resolver: &mut InstanceResolver,
        program: &Program,
        type_var_generator: &mut TypeVarGenerator,
        adt_type_info_map: &mut BTreeMap<TypeDefId, AdtTypeInfo>,
        record_type_info_map: &mut BTreeMap<TypeDefId, RecordTypeInfo>,
    ) {
        for (typedef_id, typedef) in program.typedefs.items.iter() {
            match typedef {
                TypeDef::Adt(adt) => {
                    let args: Vec<_> = adt
                        .type_args
                        .iter()
                        .map(|arg| Type::Var(*arg, Vec::new()))
                        .collect();
                    let adt_type = Type::Named(adt.name.clone(), *typedef_id, args.clone());
                    let mut variant_types = Vec::new();
                    for variant in adt.variants.iter() {
                        let mut item_types = Vec::new();
                        for item in variant.items.iter() {
                            let item_ty = process_type_signature(
                                item.type_signature_id,
                                program,
                                type_var_generator,
                            );
                            let item_ty = item_ty.remove_fixed_types();
                            let location = program
                                .type_signatures
                                .get(&item.type_signature_id)
                                .location_id;
                            item_types.push((item_ty, location));
                        }
                        variant_types.push(VariantTypeInfo {
                            item_types: item_types,
                        });
                    }
                    let mut derived_classes = Vec::new();
                    for derived_class in &adt.derived_classes {
                        let instance_ty = Type::Named(adt.name.clone(), *typedef_id, args.clone());
                        let instance_index = instance_resolver.add_auto_derived(
                            derived_class.class_id,
                            instance_ty,
                            derived_class.location_id,
                        );
                        let derive_info = DeriveInfo {
                            class_id: derived_class.class_id,
                            instance_index: instance_index,
                        };
                        derived_classes.push(derive_info);
                    }
                    adt_type_info_map.insert(
                        adt.id,
                        AdtTypeInfo {
                            adt_type: adt_type,
                            variant_types: variant_types,
                            derived_classes: derived_classes,
                        },
                    );
                }
                TypeDef::Record(record) => {
                    let args: Vec<_> = record
                        .type_args
                        .iter()
                        .map(|arg| Type::Var(*arg, Vec::new()))
                        .collect();
                    let record_type = Type::Named(record.name.clone(), *typedef_id, args.clone());
                    let mut field_types = Vec::new();
                    for field in record.fields.iter() {
                        let field_ty = process_type_signature(
                            field.type_signature_id,
                            program,
                            type_var_generator,
                        );
                        let item_ty = field_ty.remove_fixed_types();
                        let location = program
                            .type_signatures
                            .get(&field.type_signature_id)
                            .location_id;
                        field_types.push((item_ty, location));
                    }
                    let mut derived_classes = Vec::new();
                    for derived_class in &record.derived_classes {
                        let instance_ty =
                            Type::Named(record.name.clone(), *typedef_id, args.clone());
                        let instance_index = instance_resolver.add_auto_derived(
                            derived_class.class_id,
                            instance_ty,
                            derived_class.location_id,
                        );
                        let derive_info = DeriveInfo {
                            class_id: derived_class.class_id,
                            instance_index: instance_index,
                        };
                        derived_classes.push(derive_info);
                    }
                    record_type_info_map.insert(
                        record.id,
                        RecordTypeInfo {
                            record_type: record_type,
                            field_types: field_types,
                            derived_classes: derived_classes,
                        },
                    );
                }
            }
        }
    }

    fn process_classes_and_user_defined_instances(
        &self,
        program: &Program,
        type_var_generator: &mut TypeVarGenerator,
        instance_resolver: &mut InstanceResolver,
        class_types: &mut BTreeMap<ClassId, Type>,
    ) {
        for (class_id, class) in program.classes.items.iter() {
            // println!("Processing type for class {}", class.name);
            let type_signature_id = class.type_signature.expect("Class has no type signature");
            let ty = process_type_signature(type_signature_id, program, type_var_generator);
            let ty = ty.remove_fixed_types();
            let ty = ty.add_constraints(&class.constraints);
            //println!("class type {}", ty);
            class_types.insert(*class_id, ty);
        }
        for (instance_id, instance) in program.instances.items.iter() {
            let instance_ty =
                process_type_signature(instance.type_signature, program, type_var_generator);
            let instance_ty = instance_ty.remove_fixed_types();

            instance_resolver.add_user_defined(
                instance.class_id,
                instance_ty,
                *instance_id,
                instance.location_id,
            );
        }
    }

    fn register_untyped_function(
        &self,
        name: String,
        function: &Function,
        body: ExprId,
        location_id: LocationId,
        type_var_generator: &mut TypeVarGenerator,
    ) -> FunctionTypeInfo {
        let mut args = Vec::new();

        let (func_type, result_type) = create_general_function_type(
            &mut args,
            function.arg_locations.len() + function.implicit_arg_count,
            type_var_generator,
        );
        let function_type_info = FunctionTypeInfo {
            displayed_name: name,
            args: args,
            typed: false,
            result: result_type,
            function_type: func_type,
            body: Some(body),
            location_id,
        };
        function_type_info
    }

    fn process_functions(
        &self,
        program: &Program,
        type_var_generator: &mut TypeVarGenerator,
        adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
        record_type_info_map: &BTreeMap<TypeDefId, RecordTypeInfo>,
        errors: &mut Vec<TypecheckError>,
        function_type_info_store: &mut FunctionTypeInfoStore,
    ) {
        for (id, function) in &program.functions.items {
            match &function.info {
                FunctionInfo::RecordConstructor(i) => {
                    let record = program.typedefs.get(&i.type_id).get_record();
                    let record_type_info = record_type_info_map
                        .get(&i.type_id)
                        .expect("record type info not found");
                    let mut func_args = Vec::new();

                    let (func_type, result_type) = create_general_function_type(
                        &mut func_args,
                        record.fields.len(),
                        type_var_generator,
                    );

                    let mut func_type_info = FunctionTypeInfo {
                        displayed_name: format!("{}_ctor", record.name),
                        args: func_args.clone(),
                        typed: true,
                        result: result_type.clone(),
                        function_type: func_type,
                        body: None,
                        location_id: record.location_id,
                    };

                    for (index, field_type) in record_type_info.field_types.iter().enumerate() {
                        let mut unifier = Unifier::new(type_var_generator.clone());
                        let arg_type = &func_args[index];
                        unifier
                            .unify(&field_type.0, arg_type)
                            .expect("Unify failed");
                        func_type_info.apply(&unifier);
                    }

                    let mut unifier = Unifier::new(type_var_generator.clone());
                    unifier
                        .unify(&record_type_info.record_type, &result_type)
                        .expect("Unify failed");

                    func_type_info.apply(&unifier);
                    function_type_info_store.add(*id, func_type_info);
                }
                FunctionInfo::VariantConstructor(i) => {
                    let adt = program.typedefs.get(&i.type_id).get_adt();
                    let adt_type_info = adt_type_info_map
                        .get(&i.type_id)
                        .expect("Adt type info not found");

                    let variant_type_info = &adt_type_info.variant_types[i.index];

                    let mut func_args = Vec::new();

                    let (func_type, result_type) = create_general_function_type(
                        &mut func_args,
                        variant_type_info.item_types.len(),
                        type_var_generator,
                    );

                    let location = program
                        .type_signatures
                        .get(&adt.variants[i.index].type_signature_id)
                        .location_id;

                    let mut func_type_info = FunctionTypeInfo {
                        displayed_name: format!("{}/{}_ctor", adt.name, adt.variants[i.index].name),
                        args: func_args.clone(),
                        typed: true,
                        result: result_type.clone(),
                        function_type: func_type,
                        body: None,
                        location_id: location,
                    };

                    for (index, item_type) in variant_type_info.item_types.iter().enumerate() {
                        let mut unifier = Unifier::new(type_var_generator.clone());
                        let arg_type = &func_args[index];
                        unifier.unify(&item_type.0, arg_type).expect("Unify failed");
                        func_type_info.apply(&unifier);
                    }

                    let mut unifier = Unifier::new(type_var_generator.clone());
                    unifier
                        .unify(&adt_type_info.adt_type, &result_type)
                        .expect("Unify failed");

                    func_type_info.apply(&unifier);
                    function_type_info_store.add(*id, func_type_info);
                }
                FunctionInfo::Lambda(i) => {
                    let displayed_name = format!("{}", function.info);
                    let func_type_info = self.register_untyped_function(
                        displayed_name,
                        function,
                        i.body,
                        i.location_id,
                        type_var_generator,
                    );
                    function_type_info_store.add(*id, func_type_info);
                }
                FunctionInfo::NamedFunction(i) => match i.type_signature {
                    Some(type_signature) => {
                        let signature_ty =
                            process_type_signature(type_signature, program, type_var_generator);

                        let mut func_args = Vec::new();

                        let (func_type, result_type) = create_general_function_type(
                            &mut func_args,
                            function.arg_locations.len(),
                            type_var_generator,
                        );

                        let is_member = i.kind != NamedFunctionKind::Free;

                        let mut func_type_info = FunctionTypeInfo {
                            displayed_name: i.name.clone(),
                            args: func_args.clone(),
                            typed: true,
                            result: result_type.clone(),
                            function_type: func_type,
                            body: i.body,
                            location_id: i.location_id,
                        };

                        let mut unifier = Unifier::new(type_var_generator.clone());
                        if unifier
                            .unify(&signature_ty, &func_type_info.function_type)
                            .is_err()
                        {
                            let err = TypecheckError::FunctionArgAndSignatureMismatch(
                                i.name.clone(),
                                func_args.len(),
                                signature_ty.get_arg_count(),
                                i.location_id,
                                is_member,
                            );
                            errors.push(err);
                        } else {
                            func_type_info.apply(&unifier);
                        }
                        function_type_info_store.add(*id, func_type_info);
                    }
                    None => match i.body {
                        Some(body) => {
                            let displayed_name = format!("{}", function.info);
                            let func_type_info = self.register_untyped_function(
                                displayed_name,
                                function,
                                body,
                                i.location_id,
                                type_var_generator,
                            );
                            function_type_info_store.add(*id, func_type_info);
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
    }

    fn check_main(&self, program: &Program, errors: &mut Vec<TypecheckError>) {
        let mut main_found = false;

        for (_, function) in &program.functions.items {
            match &function.info {
                FunctionInfo::NamedFunction(info) => {
                    if info.module == siko_constants::MAIN_MODULE
                        && info.name == siko_constants::MAIN_FUNCTION
                    {
                        main_found = true;
                    }
                }
                _ => {}
            }
        }

        if !main_found {
            errors.push(TypecheckError::MainNotFound);
        }
    }

    fn init_expr_types(
        &self,
        function_id: &FunctionId,
        group: &DependencyGroup<FunctionId>,
        function_type_info_store: &mut FunctionTypeInfoStore,
        type_store: &mut TypeStore,
        class_member_type_info_map: &BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
        adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
        program: &Program,
        type_var_generator: TypeVarGenerator,
    ) {
        let function_type_info = function_type_info_store.get_mut(function_id);
        let body = function_type_info.body.expect("body not found");
        let mut initializer = TypeStoreInitializer::new(
            program,
            group,
            type_store,
            function_type_info_store,
            class_member_type_info_map,
            adt_type_info_map,
            type_var_generator.clone(),
        );
        walk_expr(&body, &mut initializer);
    }

    fn process_function(
        &self,
        function_id: &FunctionId,
        errors: &mut Vec<TypecheckError>,
        group: &DependencyGroup<FunctionId>,
        function_type_info_store: &mut FunctionTypeInfoStore,
        type_store: &mut TypeStore,
        program: &Program,
        type_var_generator: TypeVarGenerator,
    ) {
        //let func = program.functions.get(function_id);
        //println!("Checking {}", func.info);
        let function_type_info = function_type_info_store.get(function_id);
        let result_ty = function_type_info.result.clone();
        let body = function_type_info.body.expect("body not found");
        let mut checker = ExpressionChecker::new(
            program,
            group,
            type_store,
            type_var_generator.clone(),
            function_type_info_store,
            errors,
        );
        walk_expr(&body, &mut checker);
        checker.match_expr_with(body, &result_ty);
    }

    fn process_dep_group(
        &self,
        group: &DependencyGroup<FunctionId>,
        errors: &mut Vec<TypecheckError>,
        function_type_info_store: &mut FunctionTypeInfoStore,
        type_store: &mut TypeStore,
        class_member_type_info_map: &BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
        adt_type_info_map: &BTreeMap<TypeDefId, AdtTypeInfo>,
        program: &Program,
        type_var_generator: TypeVarGenerator,
    ) {
        for function in &group.items {
            self.init_expr_types(
                function,
                group,
                function_type_info_store,
                type_store,
                class_member_type_info_map,
                adt_type_info_map,
                program,
                type_var_generator.clone(),
            );
        }

        for _ in 0..2 {
            for function in &group.items {
                self.process_function(
                    function,
                    errors,
                    group,
                    function_type_info_store,
                    type_store,
                    program,
                    type_var_generator.clone(),
                );
            }
            if !errors.is_empty() {
                break;
            }
        }
    }

    fn process_class_members(
        &self,
        program: &Program,
        type_var_generator: &mut TypeVarGenerator,
        class_member_type_info_map: &mut BTreeMap<ClassMemberId, ClassMemberTypeInfo>,
    ) {
        for (class_member_id, class_member) in &program.class_members.items {
            let ty =
                process_type_signature(class_member.type_signature, program, type_var_generator);
            let class_member_type_info = ClassMemberTypeInfo { ty: ty };
            class_member_type_info_map.insert(*class_member_id, class_member_type_info);
        }
    }

    pub fn check(&self, program: &Program, counter: RcCounter) -> Result<(), Error> {
        let mut errors = Vec::new();
        let mut type_var_generator = TypeVarGenerator::new(counter);
        let mut class_types = BTreeMap::new();
        let mut instance_resolver = InstanceResolver::new();
        let mut adt_type_info_map = BTreeMap::new();
        let mut record_type_info_map = BTreeMap::new();
        let mut function_type_info_store = FunctionTypeInfoStore::new();
        let mut class_member_type_info_map = BTreeMap::new();

        self.process_classes_and_user_defined_instances(
            program,
            &mut type_var_generator,
            &mut instance_resolver,
            &mut class_types,
        );

        self.process_class_members(
            program,
            &mut type_var_generator,
            &mut class_member_type_info_map,
        );

        self.process_data_types(
            &mut instance_resolver,
            program,
            &mut type_var_generator,
            &mut adt_type_info_map,
            &mut record_type_info_map,
        );

        instance_resolver.check_conflicts(&mut errors, program, type_var_generator.clone());

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        self.process_derived_instances_instances(
            &mut instance_resolver,
            &mut errors,
            program,
            &mut type_var_generator,
            &adt_type_info_map,
            &record_type_info_map,
        );

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        self.process_functions(
            program,
            &mut type_var_generator,
            &adt_type_info_map,
            &record_type_info_map,
            &mut errors,
            &mut function_type_info_store,
        );

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        let function_dep_processor =
            FunctionDependencyProcessor::new(program, &function_type_info_store);

        let ordered_dep_groups = function_dep_processor.process_functions();

        self.check_main(program, &mut errors);

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        for group in &ordered_dep_groups {
            let mut type_store = TypeStore::new();
            self.process_dep_group(
                group,
                &mut errors,
                &mut function_type_info_store,
                &mut type_store,
                &class_member_type_info_map,
                &adt_type_info_map,
                program,
                type_var_generator.clone(),
            );
            //type_store.dump(program);
        }

        if !errors.is_empty() {
            return Err(Error::typecheck_err(errors));
        }

        function_type_info_store.dump(program);

        Ok(())
    }
}
