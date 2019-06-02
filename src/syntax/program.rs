use crate::location_info::item::LocationId;
use crate::syntax::class::Class;
use crate::syntax::class::ClassId;
use crate::syntax::class::Instance;
use crate::syntax::class::InstanceId;
use crate::syntax::data::Adt;
use crate::syntax::data::AdtId;
use crate::syntax::data::Record;
use crate::syntax::data::RecordFieldId;
use crate::syntax::data::RecordId;
use crate::syntax::data::Variant;
use crate::syntax::data::VariantId;
use crate::syntax::expr::Expr;
use crate::syntax::expr::ExprId;
use crate::syntax::function::Function;
use crate::syntax::function::FunctionId;
use crate::syntax::import::ImportId;
use crate::syntax::module::Module;
use crate::syntax::module::ModuleId;
use crate::syntax::pattern::Pattern;
use crate::syntax::pattern::PatternId;
use crate::syntax::types::TypeSignature;
use crate::syntax::types::TypeSignatureId;
use crate::util::Counter;
use std::collections::BTreeMap;
use crate::syntax::class::ClassMemberId;
use crate::syntax::class::ClassMember;

#[derive(Debug, Clone)]
pub struct Program {
    pub modules: BTreeMap<ModuleId, Module>,
    pub functions: BTreeMap<FunctionId, Function>,
    pub records: BTreeMap<RecordId, Record>,
    pub adts: BTreeMap<AdtId, Adt>,
    pub variants: BTreeMap<VariantId, Variant>,
    pub classes: BTreeMap<ClassId, Class>,
    pub class_members: BTreeMap<ClassMemberId, ClassMember>,
    pub instances: BTreeMap<InstanceId, Instance>,
    pub exprs: BTreeMap<ExprId, (Expr, LocationId)>,
    pub type_signatures: BTreeMap<TypeSignatureId, (TypeSignature, LocationId)>,
    pub patterns: BTreeMap<PatternId, (Pattern, LocationId)>,
    module_id: Counter,
    function_id: Counter,
    import_id: Counter,
    expr_id: Counter,
    type_signature_id: Counter,
    adt_id: Counter,
    variant_id: Counter,
    record_id: Counter,
    record_field_id: Counter,
    pattern_id: Counter,
    class_id: Counter,
    class_member_id: Counter,
    instance_id: Counter,
}

impl Program {
    pub fn new() -> Program {
        Program {
            modules: BTreeMap::new(),
            functions: BTreeMap::new(),
            records: BTreeMap::new(),
            adts: BTreeMap::new(),
            variants: BTreeMap::new(),
            classes: BTreeMap::new(),
            class_members: BTreeMap::new(),
            instances: BTreeMap::new(),
            exprs: BTreeMap::new(),
            type_signatures: BTreeMap::new(),
            patterns: BTreeMap::new(),
            module_id: Counter::new(),
            function_id: Counter::new(),
            import_id: Counter::new(),
            expr_id: Counter::new(),
            type_signature_id: Counter::new(),
            adt_id: Counter::new(),
            variant_id: Counter::new(),
            record_id: Counter::new(),
            record_field_id: Counter::new(),
            pattern_id: Counter::new(),
            class_id: Counter::new(),
            class_member_id: Counter::new(),
            instance_id: Counter::new(),
        }
    }

    pub fn get_module_id(&mut self) -> ModuleId {
        ModuleId {
            id: self.module_id.next(),
        }
    }

    pub fn get_function_id(&mut self) -> FunctionId {
        FunctionId {
            id: self.module_id.next(),
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

    pub fn get_adt_id(&mut self) -> AdtId {
        AdtId {
            id: self.adt_id.next(),
        }
    }

    pub fn get_variant_id(&mut self) -> VariantId {
        VariantId {
            id: self.variant_id.next(),
        }
    }

    pub fn get_record_id(&mut self) -> RecordId {
        RecordId {
            id: self.record_id.next(),
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

    pub fn get_instance_id(&mut self) -> InstanceId {
        InstanceId {
            id: self.instance_id.next(),
        }
    }

    pub fn add_module(&mut self, id: ModuleId, module: Module) {
        self.modules.insert(id, module);
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

    pub fn add_class(&mut self, id: ClassId, class: Class) {
        self.classes.insert(id, class);
    }

    pub fn add_class_member(&mut self, id: ClassMemberId, class_member: ClassMember) {
        self.class_members.insert(id, class_member);
    }

    pub fn add_instance(&mut self, id: InstanceId, instance: Instance) {
        self.instances.insert(id, instance);
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
