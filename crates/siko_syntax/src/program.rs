use crate::class::Class;
use crate::class::ClassId;
use crate::class::ClassMember;
use crate::class::ClassMemberId;
use crate::class::Instance;
use crate::class::InstanceId;
use crate::data::Adt;
use crate::data::AdtId;
use crate::data::Record;
use crate::data::RecordFieldId;
use crate::data::RecordId;
use crate::data::Variant;
use crate::data::VariantId;
use crate::expr::Expr;
use crate::expr::ExprId;
use crate::function::Function;
use crate::function::FunctionId;
use crate::import::ImportId;
use crate::module::Module;
use crate::module::ModuleId;
use crate::pattern::Pattern;
use crate::pattern::PatternId;
use crate::types::TypeSignature;
use crate::types::TypeSignatureId;
use siko_location_info::item::LocationId;
use siko_util::Counter;
use siko_util::ItemContainer;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Program {
    pub modules: ItemContainer<ModuleId, Module>,
    pub functions: ItemContainer<FunctionId, Function>,
    pub records: ItemContainer<RecordId, Record>,
    pub adts: ItemContainer<AdtId, Adt>,
    pub variants: ItemContainer<VariantId, Variant>,
    pub classes: ItemContainer<ClassId, Class>,
    pub class_members: ItemContainer<ClassMemberId, ClassMember>,
    pub instances: ItemContainer<InstanceId, Instance>,
    pub exprs: BTreeMap<ExprId, (Expr, LocationId)>,
    pub type_signatures: BTreeMap<TypeSignatureId, (TypeSignature, LocationId)>,
    pub patterns: BTreeMap<PatternId, (Pattern, LocationId)>,
    import_id: Counter,
    expr_id: Counter,
    type_signature_id: Counter,
    record_field_id: Counter,
    pattern_id: Counter,
}

impl Program {
    pub fn new() -> Program {
        Program {
            modules: ItemContainer::new(),
            functions: ItemContainer::new(),
            records: ItemContainer::new(),
            adts: ItemContainer::new(),
            variants: ItemContainer::new(),
            classes: ItemContainer::new(),
            class_members: ItemContainer::new(),
            instances: ItemContainer::new(),
            exprs: BTreeMap::new(),
            type_signatures: BTreeMap::new(),
            patterns: BTreeMap::new(),
            import_id: Counter::new(),
            expr_id: Counter::new(),
            type_signature_id: Counter::new(),
            record_field_id: Counter::new(),
            pattern_id: Counter::new(),
        }
    }

    pub fn get_import_id(&mut self) -> ImportId {
        ImportId {
            id: self.import_id.next(),
        }
    }

    pub fn get_expr_id(&mut self) -> ExprId {
        ExprId {
            id: self.expr_id.next(),
        }
    }

    pub fn get_record_field_id(&mut self) -> RecordFieldId {
        RecordFieldId {
            id: self.record_field_id.next(),
        }
    }

    pub fn get_type_signature_id(&mut self) -> TypeSignatureId {
        TypeSignatureId {
            id: self.type_signature_id.next(),
        }
    }

    pub fn get_pattern_id(&mut self) -> PatternId {
        PatternId {
            id: self.pattern_id.next(),
        }
    }

    pub fn add_expr(&mut self, id: ExprId, expr: Expr, location_id: LocationId) {
        self.exprs.insert(id, (expr, location_id));
    }

    pub fn add_type_signature(
        &mut self,
        id: TypeSignatureId,
        type_signature: TypeSignature,
        location_id: LocationId,
    ) {
        self.type_signatures
            .insert(id, (type_signature, location_id));
    }

    pub fn add_pattern(&mut self, id: PatternId, pattern: Pattern, location_id: LocationId) {
        self.patterns.insert(id, (pattern, location_id));
    }

    pub fn get_expr(&self, id: &ExprId) -> &Expr {
        &self.exprs.get(id).expect("Expr not found").0
    }

    pub fn get_expr_location(&self, id: &ExprId) -> LocationId {
        self.exprs.get(id).expect("Expr not found").1
    }

    pub fn get_type_signature(&self, id: &TypeSignatureId) -> &TypeSignature {
        &self
            .type_signatures
            .get(id)
            .expect("TypeSignature not found")
            .0
    }

    pub fn get_type_signature_location(&self, id: &TypeSignatureId) -> LocationId {
        self.type_signatures
            .get(id)
            .expect("TypeSignature not found")
            .1
    }
}
