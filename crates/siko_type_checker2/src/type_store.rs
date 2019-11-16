use crate::common::AdtTypeInfo;
use crate::common::RecordTypeInfo;
use crate::types::ResolverContext;
use crate::types::Type;
use crate::unifier::Unifier;
use siko_ir::expr::ExprId;
use siko_ir::pattern::PatternId;
use siko_ir::program::Program;
use std::collections::BTreeMap;
use std::fmt;

pub enum ExpressionTypeState {
    ExprType(Type),
    FunctionCall(Type, Type),
    RecordInitialization(RecordTypeInfo, Type),
}

impl ExpressionTypeState {
    pub fn apply(&mut self, unifier: &Unifier) -> bool {
        match self {
            ExpressionTypeState::ExprType(ty) => ty.apply(unifier),
            ExpressionTypeState::FunctionCall(func_ty, ty) => {
                let changed = func_ty.apply(unifier);
                let changed = ty.apply(unifier) || changed;
                changed
            }
            ExpressionTypeState::RecordInitialization(record_type_info, ty) => {
                let changed = record_type_info.apply(unifier);
                let changed = ty.apply(unifier) || changed;
                changed
            }
        }
    }
}

impl fmt::Display for ExpressionTypeState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ExpressionTypeState::ExprType(ty) => write!(f, "{}", ty),
            ExpressionTypeState::FunctionCall(ty1, ty2) => write!(f, "FC {}, {}", ty1, ty2),
            ExpressionTypeState::RecordInitialization(i, ty) => {
                write!(f, "RI {}, {}", i.record_type, ty)
            }
        }
    }
}

pub enum PatternTypeState {
    PatternType(Type),
    VariantType(AdtTypeInfo, Type),
}

impl PatternTypeState {
    pub fn apply(&mut self, unifier: &Unifier) -> bool {
        match self {
            PatternTypeState::PatternType(ty) => ty.apply(unifier),
            PatternTypeState::VariantType(adt_ty, ty) => {
                let changed = adt_ty.apply(unifier);
                let changed = ty.apply(unifier) || changed;
                changed
            }
        }
    }
}

impl fmt::Display for PatternTypeState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PatternTypeState::PatternType(ty) => write!(f, "{}", ty),
            PatternTypeState::VariantType(i, ty) => write!(f, "AI {}, {}", i.adt_type, ty),
        }
    }
}

pub struct TypeStore {
    expr_types: BTreeMap<ExprId, ExpressionTypeState>,
    pattern_types: BTreeMap<PatternId, PatternTypeState>,
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
        assert!(r.is_none());
    }

    pub fn initialize_expr_with_func(&mut self, expr_id: ExprId, ty: Type, func_ty: Type) {
        let r = self
            .expr_types
            .insert(expr_id, ExpressionTypeState::FunctionCall(func_ty, ty));
        assert!(r.is_none());
    }

    pub fn initialize_expr_with_record_type(
        &mut self,
        expr_id: ExprId,
        ty: Type,
        record_type_info: RecordTypeInfo,
    ) {
        let r = self.expr_types.insert(
            expr_id,
            ExpressionTypeState::RecordInitialization(record_type_info, ty),
        );
        assert!(r.is_none());
    }

    pub fn initialize_pattern(&mut self, pattern_id: PatternId, ty: Type) {
        self.pattern_types
            .insert(pattern_id, PatternTypeState::PatternType(ty));
    }

    pub fn initialize_pattern_with_adt_type(
        &mut self,
        pattern_id: PatternId,
        ty: Type,
        adt_type_info: AdtTypeInfo,
    ) {
        self.pattern_types
            .insert(pattern_id, PatternTypeState::VariantType(adt_type_info, ty));
    }

    pub fn get_expr_type(&self, expr_id: &ExprId) -> &Type {
        match self.expr_types.get(expr_id).expect("Expr type not found") {
            ExpressionTypeState::ExprType(ty) => ty,
            ExpressionTypeState::FunctionCall(_, ty) => ty,
            ExpressionTypeState::RecordInitialization(_, ty) => ty,
        }
    }

    pub fn get_func_type_for_expr(&self, expr_id: &ExprId) -> &Type {
        match self.expr_types.get(expr_id).expect("Expr type not found") {
            ExpressionTypeState::ExprType(_) => unreachable!(),
            ExpressionTypeState::FunctionCall(func_type, _) => func_type,
            ExpressionTypeState::RecordInitialization(..) => unreachable!(),
        }
    }

    pub fn get_record_type_info_for_expr(&self, expr_id: &ExprId) -> &RecordTypeInfo {
        match self.expr_types.get(expr_id).expect("Expr type not found") {
            ExpressionTypeState::ExprType(_) => unreachable!(),
            ExpressionTypeState::FunctionCall(..) => unreachable!(),
            ExpressionTypeState::RecordInitialization(info, _) => info,
        }
    }

    pub fn get_pattern_type(&self, pattern_id: &PatternId) -> &Type {
        match self
            .pattern_types
            .get(pattern_id)
            .expect("Pattern type not found")
        {
            PatternTypeState::PatternType(ty) => ty,
            PatternTypeState::VariantType(_, ty) => ty,
        }
    }

    pub fn get_adt_type_info_for_pattern(&self, pattern_id: &PatternId) -> &AdtTypeInfo {
        match self
            .pattern_types
            .get(pattern_id)
            .expect("Pattern type not found")
        {
            PatternTypeState::PatternType(_) => unreachable!(),
            PatternTypeState::VariantType(info, _) => info,
        }
    }

    pub fn apply(&mut self, unifier: &Unifier) {
        for (_, expr_ty) in &mut self.expr_types {
            //let old = format!("{}", expr_ty);
            if expr_ty.apply(unifier) {
                //println!("E {} -> {}", old, expr_ty);
            }
        }
        for (_, pattern_ty) in &mut self.pattern_types {
            //let old = format!("{}", pattern_ty);
            if pattern_ty.apply(unifier) {
                //println!("P {} -> {}", old, pattern_ty);
            }
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
        for (id, _) in &self.pattern_types {
            let pattern_ty = self.get_pattern_type(id);
            println!(
                "P: {}: {}",
                id,
                pattern_ty.get_resolved_type_string_with_context(&mut context)
            );
        }
    }
}
