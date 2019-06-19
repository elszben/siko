use crate::class::Class;
use crate::class::ClassId;
use crate::class::ClassMember;
use crate::class::ClassMemberId;
use crate::class::Instance;
use crate::class::InstanceId;
use crate::expr::Expr;
use crate::expr::ExprId;
use crate::function::Function;
use crate::function::FunctionId;
use crate::instance_selection_context::InstanceSelectionContext;
use crate::pattern::Pattern;
use crate::pattern::PatternId;
use crate::types::TypeDef;
use crate::types::TypeDefId;
use crate::types::TypeSignature;
use crate::types::TypeSignatureId;
use siko_location_info::item::ItemInfo;
use siko_util::ItemContainer;

#[derive(Debug)]
pub struct BuiltinTypes {
    pub int_id: Option<TypeDefId>,
    pub float_id: Option<TypeDefId>,
    pub bool_id: Option<TypeDefId>,
    pub string_id: Option<TypeDefId>,
}

#[derive(Debug)]
pub struct Program {
    pub type_signatures: ItemContainer<TypeSignatureId, ItemInfo<TypeSignature>>,
    pub exprs: ItemContainer<ExprId, ItemInfo<Expr>>,
    pub functions: ItemContainer<FunctionId, Function>,
    pub typedefs: ItemContainer<TypeDefId, TypeDef>,
    pub patterns: ItemContainer<PatternId, ItemInfo<Pattern>>,
    pub classes: ItemContainer<ClassId, Class>,
    pub class_members: ItemContainer<ClassMemberId, ClassMember>,
    pub instances: ItemContainer<InstanceId, Instance>,

    pub builtin_types: BuiltinTypes,
}

impl Program {
    pub fn new() -> Program {
        let builtin_types = BuiltinTypes {
            int_id: None,
            float_id: None,
            bool_id: None,
            string_id: None,
        };
        Program {
            type_signatures: ItemContainer::new(),
            exprs: ItemContainer::new(),
            functions: ItemContainer::new(),
            typedefs: ItemContainer::new(),
            patterns: ItemContainer::new(),
            classes: ItemContainer::new(),
            class_members: ItemContainer::new(),
            instances: ItemContainer::new(),
            builtin_types: builtin_types,
        }
    }
}
