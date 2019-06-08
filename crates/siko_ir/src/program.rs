use crate::class::Class;
use crate::class::ClassId;
use crate::class::ClassMemberId;
use crate::expr::Expr;
use crate::expr::ExprId;
use crate::function::Function;
use crate::function::FunctionId;
use crate::pattern::Pattern;
use crate::pattern::PatternId;
use crate::pattern::PatternInfo;
use crate::types::TypeDef;
use crate::types::TypeDefId;
use crate::types::TypeSignature;
use crate::types::TypeSignatureId;
use siko_location_info::item::LocationId;

use siko_util::Counter;
use siko_util::ItemContainer;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ItemInfo<T> {
    pub item: T,
    pub location_id: LocationId,
}

impl<T> ItemInfo<T> {
    pub fn new(item: T, location_id: LocationId) -> ItemInfo<T> {
        ItemInfo {
            item: item,
            location_id: location_id,
        }
    }
}

#[derive(Debug)]
pub struct Program {
    pub type_signatures: ItemContainer<TypeSignatureId, ItemInfo<TypeSignature>>,
    pub exprs: ItemContainer<ExprId, ItemInfo<Expr>>,
    pub functions: ItemContainer<FunctionId, Function>,
    pub typedefs: ItemContainer<TypeDefId, TypeDef>,
    pub patterns: BTreeMap<PatternId, PatternInfo>,
    pub classes: ItemContainer<ClassId, Class>,
    pattern_id: Counter,
    class_member_id: Counter,
}

impl Program {
    pub fn new() -> Program {
        Program {
            type_signatures: ItemContainer::new(),
            exprs: ItemContainer::new(),
            functions: ItemContainer::new(),
            typedefs: ItemContainer::new(),
            patterns: BTreeMap::new(),
            classes: ItemContainer::new(),
            pattern_id: Counter::new(),
            class_member_id: Counter::new(),
        }
    }

    pub fn get_class_member_id(&mut self) -> ClassMemberId {
        ClassMemberId {
            id: self.class_member_id.next(),
        }
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
