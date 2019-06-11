use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone)]
pub struct Counter {
    value: usize,
}

impl Counter {
    pub fn new() -> Counter {
        Counter { value: 0 }
    }

    pub fn next(&mut self) -> usize {
        let v = self.value;
        self.value += 1;
        v
    }
}

pub fn format_list<T: fmt::Display>(items: &[T]) -> String {
    let ss: Vec<_> = items.iter().map(|i| format!("{}", i)).collect();
    format!("{}", ss.join(", "))
}

#[derive(Debug)]
pub struct ItemContainer<Key, Item> {
    pub items: BTreeMap<Key, Item>,
    id: Counter,
}

impl<Key: Ord + From<usize>, Item> ItemContainer<Key, Item> {
    pub fn new() -> ItemContainer<Key, Item> {
        ItemContainer {
            items: BTreeMap::new(),
            id: Counter::new(),
        }
    }

    pub fn get_id(&mut self) -> Key {
        self.id.next().into()
    }

    pub fn add_item(&mut self, key: Key, item: Item) {
        self.items.insert(key, item);
    }

    pub fn get(&self, key: &Key) -> &Item {
        self.items.get(key).expect("Item not found")
    }

    pub fn get_mut(&mut self, key: &Key) -> &mut Item {
        self.items.get_mut(key).expect("Item not found")
    }
}

pub struct Collector<Key, Item> {
   pub  items: BTreeMap<Key, BTreeSet<Item>>
}

impl<Key: Ord, Item: Ord> Collector<Key, Item> {
    pub fn new() -> Collector<Key, Item> {
        Collector {
            items: BTreeMap::new()
        }
    }

    pub fn add(&mut self, key: Key, item: Item) {
        let entry = self.items.entry(key).or_insert_with(|| BTreeSet::new());
        entry.insert(item);
    }
}