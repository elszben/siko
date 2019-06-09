use crate::class::Class;
use crate::class::ClassId;
use crate::class::ClassMemberId;
use crate::expr::Expr;
use crate::expr::ExprId;
use crate::function::Function;
use crate::function::FunctionId;
use crate::pattern::Pattern;
use crate::pattern::PatternId;
use crate::types::TypeDef;
use crate::types::TypeDefId;
use crate::types::TypeSignature;
use crate::types::TypeSignatureId;
use siko_location_info::item::ItemInfo;

use siko_util::Counter;
use siko_util::ItemContainer;

#[derive(Debug)]
pub struct Program {
    pub type_signatures: ItemContainer<TypeSignatureId, ItemInfo<TypeSignature>>,
    pub exprs: ItemContainer<ExprId, ItemInfo<Expr>>,
    pub functions: ItemContainer<FunctionId, Function>,
    pub typedefs: ItemContainer<TypeDefId, TypeDef>,
    pub patterns: ItemContainer<PatternId, ItemInfo<Pattern>>,
    pub classes: ItemContainer<ClassId, Class>,
    class_member_id: Counter,
}

impl Program {
    pub fn new() -> Program {
        Program {
            type_signatures: ItemContainer::new(),
            exprs: ItemContainer::new(),
            functions: ItemContainer::new(),
            typedefs: ItemContainer::new(),
            patterns: ItemContainer::new(),
            classes: ItemContainer::new(),
            class_member_id: Counter::new(),
        }
    }

    pub fn get_class_member_id(&mut self) -> ClassMemberId {
        ClassMemberId {
            id: self.class_member_id.next(),
        }
    }
}
