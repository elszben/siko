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
use siko_constants::BOOL_NAME;
use siko_constants::OPTION_NAME;
use siko_constants::ORDERING_NAME;
use siko_constants::PRELUDE_NAME;
use siko_constants::STRING_NAME;
use siko_location_info::item::ItemInfo;
use siko_util::ItemContainer;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

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
        Program {
            type_signatures: ItemContainer::new(),
            exprs: ItemContainer::new(),
            functions: ItemContainer::new(),
            typedefs: ItemContainer::new(),
            patterns: ItemContainer::new(),
            classes: ItemContainer::new(),
            class_members: ItemContainer::new(),
            instances: ItemContainer::new(),
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

    pub fn to_debug_string(&self, type_id: &TypeId) -> String {
        let ty = self.types.get(type_id).expect("Type not found");
        match ty {
            Type::Function(func_type) => {
                let from = self.to_debug_string(&func_type.from);
                let to = self.to_debug_string(&func_type.to);
                format!("{} -> {}", from, to)
            }
            Type::Named(name, _, items) => {
                let items: Vec<_> = items.iter().map(|i| self.to_debug_string(i)).collect();
                if items.is_empty() {
                    format!("{}", name)
                } else {
                    format!("{} {}", name, items.join(" "))
                }
            }
            Type::Tuple(items) => {
                let items: Vec<_> = items.iter().map(|i| self.to_debug_string(i)).collect();
                format!("({})", items.join(", "))
            }
            Type::TypeArgument(index, _) => format!("{}", index),
        }
    }

    pub fn match_generic_types(
        &self,
        concrete_type: &ConcreteType,
        generic_type_id: &TypeId,
        sub_context: &mut SubstitutionContext,
    ) {
        let generic_type = self.types.get(generic_type_id).expect("Type not found");
        //println!("Matching {} and {}", concrete_type, self.to_debug_string(generic_type_id));
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
            self.get_named_type("Data.String", STRING_NAME),
            vec![],
        )
    }

    pub fn bool_concrete_type(&self) -> ConcreteType {
        ConcreteType::Named(
            BOOL_NAME.to_string(),
            self.get_named_type("Data.Bool", BOOL_NAME),
            vec![],
        )
    }

    pub fn option_concrete_type(&self, inner: ConcreteType) -> ConcreteType {
        ConcreteType::Named(
            OPTION_NAME.to_string(),
            self.get_named_type("Data.Option", OPTION_NAME),
            vec![inner],
        )
    }

    pub fn ordering_concrete_type(&self) -> ConcreteType {
        ConcreteType::Named(
            ORDERING_NAME.to_string(),
            self.get_named_type("Data.Ordering", ORDERING_NAME),
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
