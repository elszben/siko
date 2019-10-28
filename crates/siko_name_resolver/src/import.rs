use crate::item::DataMember;
use crate::item::Item;

#[derive(Debug, Clone)]
pub struct ImportedItemInfo {
    pub item: Item,
    pub source_module: String,
}

impl ImportedItemInfo {
    pub fn check_ambiguity(items: &[ImportedItemInfo]) -> (usize, usize, bool) {
        if items.len() == 2 {
            let mut adt_found = false;
            let mut variant_found = false;
            let mut adt_index = 0;
            let mut variant_index = 0;
            for (index, item) in items.iter().enumerate() {
                if let Item::Adt(..) = item.item {
                    adt_found = true;
                    adt_index = index;
                }
                if let Item::Variant(..) = item.item {
                    variant_found = true;
                    variant_index = index;
                }
            }
            if adt_found && variant_found {
                (adt_index, variant_index, false)
            } else {
                (0, 0, true)
            }
        } else {
            (0, 0, true)
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImportedMemberInfo {
    pub member: DataMember,
    pub source_module: String,
}
