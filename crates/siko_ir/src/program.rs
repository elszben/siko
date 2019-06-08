use crate::class::Class;
use crate::class::ClassId;
use crate::class::ClassMemberId;
use crate::expr::Expr;
use crate::expr::ExprId;
use crate::expr::ExprInfo;
use crate::function::Function;
use crate::function::FunctionId;
use crate::pattern::Pattern;
use crate::pattern::PatternId;
use crate::pattern::PatternInfo;
use crate::types::Adt;
use crate::types::Record;
use crate::types::TypeDef;
use crate::types::TypeDefId;
use crate::types::TypeInfo;
use crate::types::TypeSignature;
use crate::types::TypeSignatureId;
use siko_location_info::item::LocationId;

use siko_util::Counter;
use siko_util::ItemContainer;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Program {
    pub type_signatures: BTreeMap<TypeSignatureId, TypeInfo>,
    pub exprs: BTreeMap<ExprId, ExprInfo>,
    pub functions: ItemContainer<FunctionId, Function>,
    pub typedefs: ItemContainer<TypeDefId, TypeDef>,
    pub patterns: BTreeMap<PatternId, PatternInfo>,
    pub classes: BTreeMap<ClassId, Class>,
    type_signature_id: Counter,
    expr_id: Counter,
    pattern_id: Counter,
    class_id: Counter,
    class_member_id: Counter,
}

impl Program {
    pub fn new() -> Program {
        Program {
            type_signatures: BTreeMap::new(),
            exprs: BTreeMap::new(),
            functions: ItemContainer::new(),
            typedefs: ItemContainer::new(),
            patterns: BTreeMap::new(),
            classes: BTreeMap::new(),
            type_signature_id: Counter::new(),
            expr_id: Counter::new(),
            pattern_id: Counter::new(),
            class_id: Counter::new(),
            class_member_id: Counter::new(),
        }
    }

    pub fn get_type_signature_id(&mut self) -> TypeSignatureId {
        TypeSignatureId {
            id: self.type_signature_id.next(),
        }
    }

    pub fn get_expr_id(&mut self) -> ExprId {
        ExprId {
            id: self.expr_id.next(),
        }
    }

    pub fn get_class_id(&mut self) -> ClassId {
        ClassId {
            id: self.class_id.next(),
        }
    }

    pub fn get_class_member_id(&mut self) -> ClassMemberId {
        ClassMemberId {
            id: self.class_member_id.next(),
        }
    }

    pub fn add_type_signature(&mut self, id: TypeSignatureId, type_info: TypeInfo) {
        self.type_signatures.insert(id, type_info);
    }

    pub fn get_type_signature(&self, id: &TypeSignatureId) -> &TypeSignature {
        &self
            .type_signatures
            .get(id)
            .expect("TypeSignature not found")
            .type_signature
    }

    pub fn get_type_signature_location(&self, id: &TypeSignatureId) -> LocationId {
        self.type_signatures
            .get(id)
            .expect("TypeSignature not found")
            .location_id
    }

    pub fn add_expr(&mut self, id: ExprId, expr_info: ExprInfo) {
        self.exprs.insert(id, expr_info);
    }

    pub fn get_expr(&self, id: &ExprId) -> &Expr {
        &self.exprs.get(id).expect("Expr not found").expr
    }

    pub fn get_expr_location(&self, id: &ExprId) -> LocationId {
        self.exprs.get(id).expect("Expr not found").location_id
    }

    pub fn get_pattern_id(&mut self) -> PatternId {
        PatternId {
            id: self.pattern_id.next(),
        }
    }

    pub fn add_pattern(&mut self, id: PatternId, pattern_info: PatternInfo) {
        self.patterns.insert(id, pattern_info);
    }

    pub fn get_pattern(&self, id: &PatternId) -> &Pattern {
        &self.patterns.get(id).expect("Pattern not found").pattern
    }

    pub fn get_pattern_location(&self, id: &PatternId) -> LocationId {
        self.patterns
            .get(id)
            .expect("Pattern not found")
            .location_id
    }
}
