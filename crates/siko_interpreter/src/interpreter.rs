use crate::data_float;
use crate::data_int;
use crate::environment::Environment;
use crate::extern_function::ExternFunction;
use crate::util::*;
use crate::value::Callable;
use crate::value::Value;
use crate::value::ValueCore;
use siko_constants::MAIN_FUNCTION;
use siko_constants::MAIN_MODULE;
use siko_constants::OPTION_NAME;
use siko_constants::ORDERING_NAME;
use siko_constants::PRELUDE_NAME;
use siko_ir::class::ClassMemberId;
use siko_ir::expr::Expr;
use siko_ir::expr::ExprId;
use siko_ir::function::FunctionId;
use siko_ir::function::FunctionInfo;
use siko_ir::function::NamedFunctionKind;
use siko_ir::pattern::Pattern;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use siko_ir::types::Adt;
use siko_ir::types::ConcreteType;
use siko_ir::types::SubstitutionContext;
use siko_ir::types::Type;
use siko_ir::types::TypeDefId;
use siko_ir::types::TypeId;
use siko_location_info::error_context::ErrorContext;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::thread_local;

thread_local! {
    static INTERPRETER_CONTEXT: RefCell<Option<Interpreter>> = RefCell::new(None);
}

pub fn eval_with_context(eval_fn: Box<dyn FnOnce(&Interpreter) -> Value>) -> Value {
    let v = INTERPRETER_CONTEXT.with(|c| {
        let interpreter = c.borrow();
        eval_fn(interpreter.as_ref().unwrap())
    });
    v
}

#[derive(Clone)]
pub struct VariantCache {
    pub variants: BTreeMap<String, usize>,
}

impl VariantCache {
    pub fn new(adt: &Adt) -> VariantCache {
        let mut variants = BTreeMap::new();
        for (index, variant) in adt.variants.iter().enumerate() {
            variants.insert(variant.name.clone(), index);
        }
        VariantCache { variants: variants }
    }

    pub fn get_index(&self, name: &str) -> usize {
        self.variants.get(name).expect("Variant not found").clone()
    }
}

#[derive(Clone)]
pub struct TypeDefIdCache {
    pub option_id: TypeDefId,
    pub ordering_id: TypeDefId,
    pub option_variants: VariantCache,
    pub ordering_variants: VariantCache,
}

pub struct Interpreter {
    program: Program,
    error_context: ErrorContext,
    typedefid_cache: Option<TypeDefIdCache>,
    extern_functions: BTreeMap<(String, String), Box<dyn ExternFunction>>,
}

impl Interpreter {
    fn new(program: Program, error_context: ErrorContext) -> Interpreter {
        Interpreter {
            program: program,
            error_context: error_context,
            typedefid_cache: None,
            extern_functions: BTreeMap::new(),
        }
    }

    fn get_func_arg_types(&self, ty_id: &TypeId, arg_count: usize) -> (Vec<TypeId>, TypeId) {
        let (func_arg_types, return_type) =
            match self.program.types.get(ty_id).expect("type not found") {
                Type::Function(func_type) => {
                    let mut arg_types = Vec::new();
                    if arg_count == 0 {
                        (arg_types, *ty_id)
                    } else {
                        let return_type = func_type.get_arg_and_return_types(
                            &self.program,
                            &mut arg_types,
                            arg_count,
                        );
                        (arg_types, return_type)
                    }
                }
                _ => (vec![], *ty_id),
            };
        (func_arg_types, return_type)
    }

    fn get_subtitution_context(
        &self,
        func_arg_types: &[TypeId],
        args: &[Value],
        return_type: &ConcreteType,
        func_return_type: &TypeId,
    ) -> SubstitutionContext {
        let mut sub_context = SubstitutionContext::new();
        for (arg_type, func_arg_type) in args.iter().zip(func_arg_types.iter()) {
            self.program
                .match_generic_types(&arg_type.ty, func_arg_type, &mut sub_context);
        }
        self.program
            .match_generic_types(&return_type, func_return_type, &mut sub_context);
        sub_context
    }

    fn call(&self, callable_value: Value, args: Vec<Value>, expr_id: Option<ExprId>) -> Value {
        match callable_value.core {
            ValueCore::Callable(mut callable) => {
                let mut callable_func_ty = callable_value.ty;
                callable.values.extend(args);
                loop {
                    let func_info = self.program.functions.get(&callable.function_id);
                    let needed_arg_count =
                        func_info.arg_locations.len() + func_info.implicit_arg_count;
                    if needed_arg_count > callable.values.len() {
                        return Value::new(ValueCore::Callable(callable), callable_func_ty);
                    } else {
                        let rest = callable.values.split_off(needed_arg_count);
                        let mut call_args = Vec::new();
                        std::mem::swap(&mut call_args, &mut callable.values);
                        let arg_count = call_args.len();
                        let mut environment = Environment::new(
                            callable.function_id,
                            call_args,
                            func_info.implicit_arg_count,
                        );
                        callable_func_ty = callable_func_ty.get_func_type(arg_count);
                        let result = self.execute(
                            callable.function_id,
                            &mut environment,
                            expr_id,
                            &callable.sub_context,
                            callable_func_ty.clone(),
                        );
                        if !rest.is_empty() {
                            if let ValueCore::Callable(new_callable) = result.core {
                                callable = new_callable;
                                callable_func_ty = result.ty;
                                callable.values.extend(rest);
                            } else {
                                unreachable!()
                            }
                        } else {
                            return result;
                        }
                    }
                }
            }
            _ => panic!("Cannot call {:?}", callable_value),
        }
    }

    fn match_pattern(
        &self,
        pattern_id: &PatternId,
        value: &Value,
        environment: &mut Environment,
        sub_context: &SubstitutionContext,
    ) -> bool {
        let pattern = &self.program.patterns.get(pattern_id).item;
        match pattern {
            Pattern::Binding(_) => {
                environment.add(*pattern_id, value.clone());
                return true;
            }
            Pattern::Tuple(ids) => match &value.core {
                ValueCore::Tuple(vs) => {
                    for (index, id) in ids.iter().enumerate() {
                        let v = &vs[index];
                        if !self.match_pattern(id, v, environment, sub_context) {
                            return false;
                        }
                    }
                    return true;
                }
                _ => {
                    return false;
                }
            },
            Pattern::Record(p_type_id, p_ids) => match &value.core {
                ValueCore::Record(type_id, vs) => {
                    if type_id == p_type_id {
                        for (index, p_id) in p_ids.iter().enumerate() {
                            let v = &vs[index];
                            if !self.match_pattern(p_id, v, environment, sub_context) {
                                return false;
                            }
                        }
                        return true;
                    }
                    return false;
                }
                _ => {
                    return false;
                }
            },
            Pattern::Variant(p_type_id, p_index, p_ids) => match &value.core {
                ValueCore::Variant(type_id, index, vs) => {
                    if type_id == p_type_id && index == p_index {
                        for (index, p_id) in p_ids.iter().enumerate() {
                            let v = &vs[index];
                            if !self.match_pattern(p_id, v, environment, sub_context) {
                                return false;
                            }
                        }
                        return true;
                    }
                    return false;
                }
                _ => {
                    return false;
                }
            },
            Pattern::Guarded(id, guard_expr_id) => {
                if self.match_pattern(id, value, environment, sub_context) {
                    let guard_value = self.eval_expr(*guard_expr_id, environment, sub_context);
                    return guard_value.core.as_bool();
                } else {
                    return false;
                }
            }
            Pattern::Typed(id, _) => self.match_pattern(id, value, environment, sub_context),
            Pattern::Wildcard => {
                return true;
            }
            Pattern::IntegerLiteral(p_v) => {
                let r = match &value.core {
                    ValueCore::Int(v) => p_v == v,
                    _ => false,
                };
                return r;
            }
            Pattern::FloatLiteral(p_v) => {
                let r = match &value.core {
                    ValueCore::Float(v) => p_v == v,
                    _ => false,
                };
                return r;
            }
            Pattern::StringLiteral(p_v) => {
                let r = match &value.core {
                    ValueCore::String(v) => p_v == v,
                    _ => false,
                };
                return r;
            }
            Pattern::BoolLiteral(p_v) => {
                let r = match &value.core {
                    ValueCore::Bool(v) => p_v == v,
                    _ => false,
                };
                return r;
            }
        }
    }

    fn call_show(&self, arg: Value) -> String {
        let string_ty = self.program.string_concrete_type();
        let class_id = self
            .program
            .class_names
            .get("Show")
            .expect("Show not found");
        let class = self.program.classes.get(class_id);
        let class_member_id = class.members.get("show").expect("show not found");
        let v = self.call_class_member(class_member_id, vec![arg], None, string_ty);
        if let ValueCore::String(s) = v.core {
            return s;
        } else {
            unreachable!();
        }
    }

    pub fn call_op_eq(&self, arg1: Value, arg2: Value) -> Value {
        let bool_ty = self.program.bool_concrete_type();
        let class_id = self
            .program
            .class_names
            .get("PartialEq")
            .expect("PartialEq not found");
        let class = self.program.classes.get(class_id);
        let class_member_id = class.members.get("opEq").expect("opEq not found");
        self.call_class_member(class_member_id, vec![arg1, arg2], None, bool_ty)
    }

    pub fn call_op_partial_cmp(&self, arg1: Value, arg2: Value) -> Value {
        let option_ordering_ty = self
            .program
            .option_concrete_type(self.program.ordering_concrete_type());
        let class_id = self
            .program
            .class_names
            .get("PartialOrd")
            .expect("PartialOrd not found");
        let class = self.program.classes.get(class_id);
        let class_member_id = class
            .members
            .get("partialCmp")
            .expect("partialCmp not found");
        self.call_class_member(class_member_id, vec![arg1, arg2], None, option_ordering_ty)
    }

    pub fn call_op_cmp(&self, arg1: Value, arg2: Value) -> Value {
        let ordering_ty = self.program.ordering_concrete_type();
        let class_id = self.program.class_names.get("Ord").expect("Ord not found");
        let class = self.program.classes.get(class_id);
        let class_member_id = class.members.get("cmp").expect("cmp not found");
        self.call_class_member(class_member_id, vec![arg1, arg2], None, ordering_ty)
    }

    fn call_class_member(
        &self,
        class_member_id: &ClassMemberId,
        arg_values: Vec<Value>,
        expr_id: Option<ExprId>,
        expr_ty: ConcreteType,
    ) -> Value {
        let member = self.program.class_members.get(class_member_id);
        let (class_member_type_id, class_arg_ty_id) = self
            .program
            .class_member_types
            .get(class_member_id)
            .expect("untyped class member");
        let (func_arg_types, return_type) =
            self.get_func_arg_types(class_member_type_id, arg_values.len());
        let callee_sub_context = self.get_subtitution_context(
            &func_arg_types[..],
            &arg_values[..],
            &expr_ty,
            &return_type,
        );
        let instance_selector_ty = self
            .program
            .to_concrete_type(class_arg_ty_id, &callee_sub_context);
        let concrete_function_type = self
            .program
            .to_concrete_type(class_member_type_id, &callee_sub_context);
        //println!("instance selector {} {}", instance_selector_ty, member.name);
        let resolver = self.program.type_instance_resolver.borrow();
        if let Some(instances) = resolver.instance_map.get(&member.class_id) {
            if let Some(instance_id) = instances.get(&instance_selector_ty) {
                let instance = self.program.instances.get(instance_id);
                let member_function_id =
                    if let Some(instance_member) = instance.members.get(&member.name) {
                        instance_member.function_id
                    } else {
                        member
                            .default_implementation
                            .expect("Default implementation not found")
                    };
                let callable = Value::new(
                    ValueCore::Callable(Callable {
                        function_id: member_function_id,
                        values: vec![],
                        sub_context: callee_sub_context,
                    }),
                    concrete_function_type,
                );
                return self.call(callable, arg_values, expr_id);
            } else {
                for (a, b) in instances {
                    println!("{} {}", a, b);
                }
                panic!("Did not find {}", instance_selector_ty);
            }
        } else {
            unreachable!()
        }
    }

    fn eval_expr(
        &self,
        expr_id: ExprId,
        environment: &mut Environment,
        sub_context: &SubstitutionContext,
    ) -> Value {
        let expr = &self.program.exprs.get(&expr_id).item;
        //println!("Eval {} {}", expr_id, expr);
        let expr_ty_id = self
            .program
            .expr_types
            .get(&expr_id)
            .expect("Untyped expr")
            .clone();
        let expr_ty = self.program.to_concrete_type(&expr_ty_id, sub_context);
        match expr {
            Expr::IntegerLiteral(v) => Value::new(ValueCore::Int(*v), expr_ty),
            Expr::StringLiteral(v) => Value::new(ValueCore::String(v.clone()), expr_ty),
            Expr::FloatLiteral(v) => Value::new(ValueCore::Float(v.clone()), expr_ty),
            Expr::BoolLiteral(v) => Value::new(ValueCore::Bool(v.clone()), expr_ty),
            Expr::ArgRef(arg_ref) => {
                return environment.get_arg(arg_ref);
            }
            Expr::StaticFunctionCall(function_id, args) => {
                let func_ty = self
                    .program
                    .function_types
                    .get(function_id)
                    .expect("untyped func");
                let (func_arg_types, return_type) = self.get_func_arg_types(func_ty, args.len());
                let arg_values: Vec<_> = args
                    .iter()
                    .map(|arg| self.eval_expr(*arg, environment, sub_context))
                    .collect();
                let callee_sub_context = self.get_subtitution_context(
                    &func_arg_types[..],
                    &arg_values[..],
                    &expr_ty,
                    &return_type,
                );
                let concrete_function_type =
                    self.program.to_concrete_type(func_ty, &callee_sub_context);
                let callable = Value::new(
                    ValueCore::Callable(Callable {
                        function_id: *function_id,
                        values: vec![],
                        sub_context: callee_sub_context,
                    }),
                    concrete_function_type,
                );
                return self.call(callable, arg_values, Some(expr_id));
            }
            Expr::DynamicFunctionCall(function_expr_id, args) => {
                let function_expr_id = self.eval_expr(*function_expr_id, environment, sub_context);
                let arg_values: Vec<_> = args
                    .iter()
                    .map(|arg| self.eval_expr(*arg, environment, sub_context))
                    .collect();
                return self.call(function_expr_id, arg_values, Some(expr_id));
            }
            Expr::Do(exprs) => {
                let mut environment = Environment::block_child(environment);
                let mut result = Value::new(ValueCore::Tuple(vec![]), expr_ty);
                assert!(!exprs.is_empty());
                for expr in exprs {
                    result = self.eval_expr(*expr, &mut environment, sub_context);
                }
                return result;
            }
            Expr::Bind(pattern_id, expr_id) => {
                let value = self.eval_expr(*expr_id, environment, sub_context);
                let r = self.match_pattern(pattern_id, &value, environment, sub_context);
                assert!(r);
                return Value::new(ValueCore::Tuple(vec![]), expr_ty);
            }
            Expr::ExprValue(_, pattern_id) => {
                return environment.get_value(pattern_id);
            }
            Expr::If(cond, true_branch, false_branch) => {
                let cond_value = self.eval_expr(*cond, environment, sub_context);
                if cond_value.core.as_bool() {
                    return self.eval_expr(*true_branch, environment, sub_context);
                } else {
                    return self.eval_expr(*false_branch, environment, sub_context);
                }
            }
            Expr::Tuple(exprs) => {
                let values: Vec<_> = exprs
                    .iter()
                    .map(|e| self.eval_expr(*e, environment, sub_context))
                    .collect();
                return Value::new(ValueCore::Tuple(values), expr_ty);
            }
            Expr::List(exprs) => {
                let values: Vec<_> = exprs
                    .iter()
                    .map(|e| self.eval_expr(*e, environment, sub_context))
                    .collect();
                return Value::new(ValueCore::List(values), expr_ty);
            }
            Expr::TupleFieldAccess(index, tuple) => {
                let tuple_value = self.eval_expr(*tuple, environment, sub_context);
                if let ValueCore::Tuple(t) = &tuple_value.core {
                    return t[*index].clone();
                } else {
                    unreachable!()
                }
            }
            Expr::Formatter(fmt, args) => {
                let subs: Vec<_> = fmt.split("{}").collect();
                let values: Vec<_> = args
                    .iter()
                    .map(|e| self.eval_expr(*e, environment, sub_context))
                    .collect();
                let mut result = String::new();
                for (index, sub) in subs.iter().enumerate() {
                    result += sub;
                    if values.len() > index {
                        let value_as_string = self.call_show(values[index].clone());
                        result += &value_as_string;
                    }
                }
                return Value::new(ValueCore::String(result), expr_ty);
            }
            Expr::FieldAccess(infos, record_expr) => {
                let record = self.eval_expr(*record_expr, environment, sub_context);
                let (id, values) = if let ValueCore::Record(id, values) = &record.core {
                    (id, values)
                } else {
                    unreachable!()
                };
                for info in infos {
                    if info.record_id != *id {
                        continue;
                    }
                    return values[info.index].clone();
                }
                unreachable!()
            }
            Expr::CaseOf(body, cases) => {
                let case_value = self.eval_expr(*body, environment, sub_context);
                for case in cases {
                    let mut case_env = Environment::block_child(environment);
                    if self.match_pattern(&case.pattern_id, &case_value, &mut case_env, sub_context)
                    {
                        let val = self.eval_expr(case.body, &mut case_env, sub_context);
                        return val;
                    }
                }
                unreachable!()
            }
            Expr::RecordInitialization(type_id, items) => {
                let mut values: Vec<_> = Vec::with_capacity(items.len());
                for _ in 0..items.len() {
                    values.push(Value::new(ValueCore::Bool(false), expr_ty.clone())); // dummy value
                }
                for item in items {
                    let value = self.eval_expr(item.expr_id, environment, sub_context);
                    values[item.index] = value;
                }
                return Value::new(ValueCore::Record(*type_id, values), expr_ty);
            }
            Expr::RecordUpdate(record_expr_id, updates) => {
                let value = self.eval_expr(*record_expr_id, environment, sub_context);
                if let ValueCore::Record(id, mut values) = value.core {
                    for update in updates {
                        if id == update.record_id {
                            for item in &update.items {
                                let value = self.eval_expr(item.expr_id, environment, sub_context);
                                values[item.index] = value;
                            }
                            return Value::new(ValueCore::Record(id, values), expr_ty);
                        }
                    }
                }
                unreachable!()
            }
            Expr::ClassFunctionCall(class_member_id, args) => {
                let arg_values: Vec<_> = args
                    .iter()
                    .map(|e| self.eval_expr(*e, environment, sub_context))
                    .collect();
                return self.call_class_member(class_member_id, arg_values, Some(expr_id), expr_ty);
            }
        }
    }

    fn call_extern(
        &self,
        module: &str,
        name: &str,
        environment: &mut Environment,
        current_expr: Option<ExprId>,
        kind: &NamedFunctionKind,
        ty: ConcreteType,
    ) -> Value {
        fn get_instance_name_from_kind(kind: &NamedFunctionKind) -> &str {
            if let NamedFunctionKind::InstanceMember(Some(s)) = kind {
                s.as_ref()
            } else {
                unreachable!()
            }
        }

        if let Some(f) = self
            .extern_functions
            .get(&(module.to_string(), name.to_string()))
        {
            return f.call(environment, current_expr, kind, ty);
        }

        match (module, name) {
            (PRELUDE_NAME, "opAdd") => {
                let instance_name = get_instance_name_from_kind(kind);
                match instance_name {
                    "StringAdd" => {
                        let l = environment.get_arg_by_index(0).core.as_string();
                        let r = environment.get_arg_by_index(1).core.as_string();
                        return Value::new(ValueCore::String(l + &r), ty);
                    }
                    _ => {
                        panic!("Unimplemented add function {}/{}", module, instance_name);
                    }
                }
            }
            (PRELUDE_NAME, "opEq") => {
                let instance_name = get_instance_name_from_kind(kind);
                match instance_name {
                    "BoolEq" => {
                        let l = environment.get_arg_by_index(0).core.as_bool();
                        let r = environment.get_arg_by_index(1).core.as_bool();
                        return Value::new(ValueCore::Bool(l == r), ty);
                    }
                    "StringEq" => {
                        let l = environment.get_arg_by_index(0).core.as_string();
                        let r = environment.get_arg_by_index(1).core.as_string();
                        return Value::new(ValueCore::Bool(l == r), ty);
                    }
                    "OrderingEq" => {
                        let l = environment
                            .get_arg_by_index(0)
                            .core
                            .as_simple_enum_variant();
                        let r = environment
                            .get_arg_by_index(1)
                            .core
                            .as_simple_enum_variant();
                        return Value::new(ValueCore::Bool(l == r), ty);
                    }
                    _ => {
                        panic!("Unimplemented eq function {}/{}", module, instance_name);
                    }
                }
            }
            (PRELUDE_NAME, "partialCmp") => {
                let instance_name = get_instance_name_from_kind(kind);
                match instance_name {
                    "StringPartialOrd" => {
                        let l = environment.get_arg_by_index(0).core.as_string();
                        let r = environment.get_arg_by_index(1).core.as_string();
                        let ord = l.partial_cmp(&r);
                        return get_opt_ordering_value(ord);
                    }
                    _ => {
                        panic!(
                            "Unimplemented partial cmp function {}/{}",
                            module, instance_name
                        );
                    }
                }
            }
            (PRELUDE_NAME, "cmp") => {
                let instance_name = get_instance_name_from_kind(kind);
                match instance_name {
                    "StringOrd" => {
                        let l = environment.get_arg_by_index(0).core.as_string();
                        let r = environment.get_arg_by_index(1).core.as_string();
                        let ord = l.cmp(&r);
                        return get_ordering_value(ord);
                    }
                    _ => {
                        panic!(
                            "Unimplemented partial cmp function {}/{}",
                            module, instance_name
                        );
                    }
                }
            }
            ("Std.Util", "assert") => {
                let v = environment.get_arg_by_index(0).core.as_bool();
                if !v {
                    let current_expr = current_expr.expect("No current expr");
                    let location_id = self.program.exprs.get(&current_expr).location_id;
                    self.error_context
                        .report_error(format!("Assertion failed"), location_id);
                    panic!("Abort not implemented");
                }
                return Value::new(ValueCore::Tuple(vec![]), ty);
            }
            (PRELUDE_NAME, "print") => {
                let v = environment.get_arg_by_index(0).core.as_string();
                print!("{}", v);
                return Value::new(ValueCore::Tuple(vec![]), ty);
            }
            (PRELUDE_NAME, "println") => {
                let v = environment.get_arg_by_index(0).core.as_string();
                println!("{}", v);
                return Value::new(ValueCore::Tuple(vec![]), ty);
            }
            (PRELUDE_NAME, "opAnd") => {
                let l = environment.get_arg_by_index(0).core.as_bool();
                let r = environment.get_arg_by_index(1).core.as_bool();
                return Value::new(ValueCore::Bool(l && r), ty);
            }
            (PRELUDE_NAME, "opOr") => {
                let l = environment.get_arg_by_index(0).core.as_bool();
                if l {
                    return Value::new(ValueCore::Bool(l), ty);
                } else {
                    let r = environment.get_arg_by_index(1).core.as_bool();
                    return Value::new(ValueCore::Bool(r), ty);
                }
            }
            (PRELUDE_NAME, "show") => {
                let instance_name = get_instance_name_from_kind(kind);
                match instance_name {
                    "ListShow" => {
                        let list = environment.get_arg_by_index(0);
                        if let ValueCore::List(items) = list.core {
                            let mut subs = Vec::new();
                            for item in items {
                                let s = self.call_show(item);
                                subs.push(s);
                            }
                            return Value::new(
                                ValueCore::String(format!("({})", subs.join(", "))),
                                ty,
                            );
                        } else {
                            unreachable!()
                        }
                    }
                    _ => {
                        panic!("Unimplemented show function {}/{}", module, instance_name);
                    }
                }
            }
            ("Data.Map", "empty") => {
                return Value::new(ValueCore::Map(BTreeMap::new()), ty);
            }
            ("Data.Map", "insert") => {
                let mut first_arg = environment.get_arg_by_index(0);
                let mut map_type_args = first_arg.ty.get_type_args();
                let mut map = first_arg.core.as_map();
                let key = environment.get_arg_by_index(1);
                let value = environment.get_arg_by_index(2);
                let res = map.insert(key, value);
                let v = match res {
                    Some(v) => create_some(v),
                    None => create_none(map_type_args.remove(1)),
                };
                first_arg.core = ValueCore::Map(map);
                let tuple = Value::new(ValueCore::Tuple(vec![first_arg, v]), ty);
                return tuple;
            }
            ("Data.Map", "get") => {
                let first_arg = environment.get_arg_by_index(0);
                let mut map_type_args = first_arg.ty.get_type_args();
                let map = first_arg.core.as_map();
                let key = environment.get_arg_by_index(1);
                let res = map.get(&key);
                let v = match res {
                    Some(v) => create_some(v.clone()),
                    None => create_none(map_type_args.remove(1)),
                };
                return v;
            }
            _ => {
                panic!("Unimplemented extern function {}/{}", module, name);
            }
        }
    }

    fn execute(
        &self,
        id: FunctionId,
        environment: &mut Environment,
        current_expr: Option<ExprId>,
        sub_context: &SubstitutionContext,
        expr_ty: ConcreteType,
    ) -> Value {
        let function = self.program.functions.get(&id);
        match &function.info {
            FunctionInfo::NamedFunction(info) => match info.body {
                Some(body) => {
                    return self.eval_expr(body, environment, sub_context);
                }
                None => {
                    return self.call_extern(
                        &info.module,
                        &info.name,
                        environment,
                        current_expr,
                        &info.kind,
                        expr_ty,
                    );
                }
            },
            FunctionInfo::Lambda(info) => {
                return self.eval_expr(info.body, environment, sub_context);
            }
            FunctionInfo::VariantConstructor(info) => {
                let adt = self.program.typedefs.get(&info.type_id).get_adt();
                let variant = &adt.variants[info.index];
                let mut values = Vec::new();
                for index in 0..variant.items.len() {
                    let v = environment.get_arg_by_index(index);
                    values.push(v);
                }
                return Value::new(
                    ValueCore::Variant(info.type_id, info.index, values),
                    expr_ty,
                );
            }
            FunctionInfo::RecordConstructor(info) => {
                let record = self.program.typedefs.get(&info.type_id).get_record();
                let mut values = Vec::new();
                for index in 0..record.fields.len() {
                    let v = environment.get_arg_by_index(index);
                    values.push(v);
                }
                return Value::new(ValueCore::Record(info.type_id, values), expr_ty);
            }
        }
    }

    fn build_typedefid_cache(&mut self) {
        let option = self.program.get_adt_by_name(PRELUDE_NAME, OPTION_NAME);
        let ordering = self.program.get_adt_by_name(PRELUDE_NAME, ORDERING_NAME);
        let cache = TypeDefIdCache {
            option_id: option.id,
            ordering_id: ordering.id,
            option_variants: VariantCache::new(option),
            ordering_variants: VariantCache::new(ordering),
        };
        self.typedefid_cache = Some(cache);
    }

    pub fn get_typedef_id_cache() -> TypeDefIdCache {
        INTERPRETER_CONTEXT.with(|i| {
            let i = i.borrow();
;            i.as_ref().expect(".").typedefid_cache.clone().expect("..")
        })
    }

    fn execute_main(interpreter: &Interpreter) -> Value {
        for (id, function) in &interpreter.program.functions.items {
            match &function.info {
                FunctionInfo::NamedFunction(info) => {
                    if info.module == MAIN_MODULE && info.name == MAIN_FUNCTION {
                        let mut environment = Environment::new(*id, vec![], 0);
                        let sub_context = SubstitutionContext::new();
                        return interpreter.execute(
                            *id,
                            &mut environment,
                            None,
                            &sub_context,
                            ConcreteType::Tuple(vec![]),
                        );
                    }
                }
                _ => {}
            }
        }

        panic!(
            "Cannot find function {} in module {}",
            MAIN_FUNCTION, MAIN_MODULE
        );
    }

    pub fn add_extern_function(
        &mut self,
        module: &str,
        name: &str,
        extern_function: Box<dyn ExternFunction>,
    ) {
        self.extern_functions
            .insert((module.to_string(), name.to_string()), extern_function);
    }

    pub fn run(program: Program, error_context: ErrorContext) -> Value {
        let mut interpreter = Interpreter::new(program, error_context);
        data_int::register_extern_functions(&mut interpreter);
        data_float::register_extern_functions(&mut interpreter);
        interpreter.build_typedefid_cache();
        INTERPRETER_CONTEXT.with(|c| {
            let mut p = c.borrow_mut();
            *p = Some(interpreter);
        });
        eval_with_context(Box::new(&Interpreter::execute_main))
    }
}
