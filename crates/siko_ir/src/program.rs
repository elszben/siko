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
use crate::pattern::Pattern;
use crate::pattern::PatternId;
use crate::types::Adt;
use crate::types::ConcreteType;
use crate::types::SubstitutionContext;
use crate::types::Type;
use crate::types::TypeDef;
use crate::types::TypeDefId;
use crate::types::TypeId;
use crate::types::TypeInstanceResolver;
use crate::types::TypeSignature;
use crate::types::TypeSignatureId;
use siko_constants::PRELUDE_NAME;
use siko_constants::STRING_NAME;
use siko_location_info::item::ItemInfo;
use siko_util::ItemContainer;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

#[derive(Debug)]
pub struct BuiltinTypes {
    pub int_id: Option<TypeDefId>,
    pub float_id: Option<TypeDefId>,
    pub bool_id: Option<TypeDefId>,
    pub string_id: Option<TypeDefId>,
    pub list_id: Option<TypeDefId>,
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
    pub type_instance_resolver: Rc<RefCell<TypeInstanceResolver>>,
    pub types: BTreeMap<TypeId, Type>,
    pub expr_types: BTreeMap<ExprId, TypeId>,
    pub function_types: BTreeMap<FunctionId, TypeId>,
    pub class_member_types: BTreeMap<ClassMemberId, (TypeId, TypeId)>,
    pub class_names: BTreeMap<String, ClassId>,
    pub named_types: BTreeMap<String, BTreeMap<String, TypeDefId>>,
}

impl Program {
    pub fn new() -> Program {
        let builtin_types = BuiltinTypes {
            int_id: None,
            float_id: None,
            bool_id: None,
            string_id: None,
            list_id: None,
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
            type_instance_resolver: Rc::new(RefCell::new(TypeInstanceResolver::new())),
            types: BTreeMap::new(),
            expr_types: BTreeMap::new(),
            function_types: BTreeMap::new(),
            class_member_types: BTreeMap::new(),
            class_names: BTreeMap::new(),
            named_types: BTreeMap::new(),
        }
    }

    pub fn to_concrete_type(
        &self,
        type_id: &TypeId,
        context: &SubstitutionContext,
    ) -> ConcreteType {
        let ty = self.types.get(type_id).expect("Type not found");
        match ty {
            Type::Function(func_type) => {
                let from = self.to_concrete_type(&func_type.from, context);
                let to = self.to_concrete_type(&func_type.to, context);
                ConcreteType::Function(Box::new(from), Box::new(to))
            }
            Type::Named(name, id, items) => {
                let items = items
                    .iter()
                    .map(|i| self.to_concrete_type(i, context))
                    .collect();
                ConcreteType::Named(name.clone(), id.clone(), items)
            }
            Type::Tuple(items) => {
                let items = items
                    .iter()
                    .map(|i| self.to_concrete_type(i, context))
                    .collect();
                ConcreteType::Tuple(items)
            }
            Type::TypeArgument(index, _) => context.get_type_id(index).clone(),
        }
    }

    pub fn match_generic_types(
        &self,
        concrete_type: &ConcreteType,
        generic_type_id: &TypeId,
        sub_context: &mut SubstitutionContext,
    ) {
        let generic_type = self.types.get(generic_type_id).expect("Type not found");
        match (concrete_type, generic_type) {
            (_, Type::TypeArgument(index, _)) => {
                sub_context.add_generic(*index, concrete_type.clone());
            }
            (ConcreteType::Function(from, to), Type::Function(f2)) => {
                self.match_generic_types(from, &f2.from, sub_context);
                self.match_generic_types(to, &f2.to, sub_context);
            }
            (ConcreteType::Named(_, _, sub_types1), Type::Named(_, _, sub_types2)) => {
                for (sub_ty1, sub_ty2) in sub_types1.iter().zip(sub_types2.iter()) {
                    self.match_generic_types(sub_ty1, sub_ty2, sub_context);
                }
            }
            (ConcreteType::Tuple(sub_types1), Type::Tuple(sub_types2)) => {
                for (sub_ty1, sub_ty2) in sub_types1.iter().zip(sub_types2.iter()) {
                    self.match_generic_types(sub_ty1, sub_ty2, sub_context);
                }
            }
            _ => panic!("{}, {:?}", concrete_type, generic_type),
        }
    }

    pub fn string_concrete_type(&self) -> ConcreteType {
        ConcreteType::Named(
            STRING_NAME.to_string(),
            self.get_named_type(PRELUDE_NAME, STRING_NAME),
            vec![],
        )
    }

    pub fn get_adt_by_name(&self, module: &str, name: &str) -> &Adt {
        let id = self
            .named_types
            .get(module)
            .expect("Module not found")
            .get(name)
            .expect("Typedef not found");
        if let TypeDef::Adt(adt) = self.typedefs.get(id) {
            adt
        } else {
            unreachable!()
        }
    }

    pub fn get_named_type(&self, module: &str, name: &str) -> TypeDefId {
        self.named_types
            .get(module)
            .expect("Module not found")
            .get(name)
            .expect("Typedef not found")
            .clone()
    }
}
