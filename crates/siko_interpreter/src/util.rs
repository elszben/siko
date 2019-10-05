use crate::value::Value;
use crate::value::ValueCore;
use siko_constants::OPTION_NAME;
use siko_constants::ORDERING_NAME;
use siko_ir::types::ConcreteType;
use std::cmp::Ordering;
use crate::interpreter::Interpreter;

 pub fn get_opt_ordering_value(ordering: Option<Ordering>) -> Value {
        match ordering {
            Some(ordering) => {
                let value = get_ordering_value(ordering);
                return create_some(value);
            }
            None => {
                let value = create_ordering(0);
                return create_none(value.ty);
            }
        }
    }


    pub fn create_some( value: Value) -> Value {
        let cache = Interpreter:: get_typedef_id_cache();
        let concrete_type = ConcreteType::Named(
            OPTION_NAME.to_string(),
            cache.option_id,
            vec![value.ty.clone()],
        );
        let core = ValueCore::Variant(
            cache.option_id,
            cache.option_variants.get_index("Some"),
            vec![value],
        );
        let some_value = Value::new(core, concrete_type);
        some_value
    }

    pub fn create_none(value_ty: ConcreteType) -> Value {
        let cache = Interpreter:: get_typedef_id_cache();
        let concrete_type =
            ConcreteType::Named(OPTION_NAME.to_string(), cache.option_id, vec![value_ty]);
        let core = ValueCore::Variant(
            cache.option_id,
            cache.option_variants.get_index("None"),
            vec![],
        );
        let none_value = Value::new(core, concrete_type);
        none_value
    }

    pub fn create_ordering( index: usize) -> Value {
        let cache = Interpreter:: get_typedef_id_cache();
        let concrete_type =
            ConcreteType::Named(ORDERING_NAME.to_string(), cache.ordering_id, vec![]);
        let core = ValueCore::Variant(cache.ordering_id, index, vec![]);
        let value = Value::new(core, concrete_type);
        value
    }
 
pub    fn get_ordering_value( ordering: Ordering) -> Value {
        let cache = Interpreter:: get_typedef_id_cache();
        match ordering {
            Ordering::Less => create_ordering(cache.ordering_variants.get_index("Less")),
            Ordering::Equal => create_ordering(cache.ordering_variants.get_index("Equal")),
            Ordering::Greater => create_ordering(cache.ordering_variants.get_index("Greater")),
        }
    }