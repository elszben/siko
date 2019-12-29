use crate::expr::Expr;
use crate::expr::ExprId;
use crate::function::Function;
use crate::function::FunctionId;
use crate::pattern::Pattern;
use crate::pattern::PatternId;
use siko_location_info::item::ItemInfo;
use siko_util::ItemContainer;

pub struct Program {
    pub exprs: ItemContainer<ExprId, ItemInfo<Expr>>,
    pub patterns: ItemContainer<PatternId, ItemInfo<Pattern>>,
    pub functions: ItemContainer<FunctionId, Function>,
}

impl Program {
    pub fn new() -> Program {
        Program {
            exprs: ItemContainer::new(),
            patterns: ItemContainer::new(),
            functions: ItemContainer::new(),
        }
    }
}
