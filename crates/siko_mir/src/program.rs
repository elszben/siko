use crate::data::TypeDef;
use crate::data::TypeDefId;
use crate::expr::Expr;
use crate::expr::ExprId;
use crate::function::Function;
use crate::function::FunctionId;
use crate::pattern::Pattern;
use crate::pattern::PatternId;
use crate::types::Type;
use siko_location_info::item::ItemInfo;
use siko_location_info::location_id::LocationId;
use siko_util::ItemContainer;
use std::collections::BTreeMap;

pub struct Program {
    pub exprs: ItemContainer<ExprId, ItemInfo<Expr>>,
    pub expr_types: BTreeMap<ExprId, Type>,
    pub patterns: ItemContainer<PatternId, ItemInfo<Pattern>>,
    pub pattern_types: BTreeMap<PatternId, Type>,
    pub functions: ItemContainer<FunctionId, Function>,
    pub typedefs: ItemContainer<TypeDefId, TypeDef>,
}

impl Program {
    pub fn new() -> Program {
        Program {
            exprs: ItemContainer::new(),
            expr_types: BTreeMap::new(),
            patterns: ItemContainer::new(),
            pattern_types: BTreeMap::new(),
            functions: ItemContainer::new(),
            typedefs: ItemContainer::new(),
        }
    }

    pub fn add_expr(&mut self, expr: Expr, location_id: LocationId, ty: Type) -> ExprId {
        let expr_info = ItemInfo {
            item: expr,
            location_id: location_id,
        };
        let expr_id = self.exprs.get_id();
        self.exprs.add_item(expr_id, expr_info);
        self.expr_types.insert(expr_id, ty);
        expr_id
    }

    pub fn add_pattern(
        &mut self,
        pattern: Pattern,
        location_id: LocationId,
        ty: Type,
    ) -> PatternId {
        let pattern_info = ItemInfo {
            item: pattern,
            location_id: location_id,
        };
        let pattern_id = self.patterns.get_id();
        self.patterns.add_item(pattern_id, pattern_info);
        self.pattern_types.insert(pattern_id, ty);
        pattern_id
    }
}
