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
use siko_location_info::item::ItemInfo;
use siko_util::Counter;
use siko_util::ItemContainer;

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
    pub exprs: ItemContainer<ExprId, ItemInfo<Expr>>,
    pub type_signatures: ItemContainer<TypeSignatureId, ItemInfo<TypeSignature>>,
    pub patterns: ItemContainer<PatternId, ItemInfo<Pattern>>,
    import_id: Counter,
    record_field_id: Counter,
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
            exprs: ItemContainer::new(),
            type_signatures: ItemContainer::new(),
            patterns: ItemContainer::new(),
            import_id: Counter::new(),
            record_field_id: Counter::new(),
        }
    }

    pub fn get_import_id(&mut self) -> ImportId {
        ImportId {
            id: self.import_id.next(),
        }
    }

    pub fn get_record_field_id(&mut self) -> RecordFieldId {
        RecordFieldId {
            id: self.record_field_id.next(),
        }
    }
}
