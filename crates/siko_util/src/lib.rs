use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fmt;
use std::rc::Rc;
use std::thread_local;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct RcCounter {
    c: Rc<RefCell<Counter>>,
}

impl RcCounter {
    pub fn new() -> RcCounter {
        RcCounter {
            c: Rc::new(RefCell::new(Counter::new())),
        }
    }

    pub fn next(&self) -> usize {
        let mut b = self.c.borrow_mut();
        b.next()
    }
}

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
    pub items: BTreeMap<Key, BTreeSet<Item>>,
}

impl<Key: Ord, Item: Ord> Collector<Key, Item> {
    pub fn new() -> Collector<Key, Item> {
        Collector {
            items: BTreeMap::new(),
        }
    }

    pub fn add_empty(&mut self, key: Key) {
        self.items.entry(key).or_insert_with(|| BTreeSet::new());
    }

    pub fn add(&mut self, key: Key, item: Item) {
        let entry = self.items.entry(key).or_insert_with(|| BTreeSet::new());
        entry.insert(item);
    }
}

pub struct ElapsedTimeMeasure {
    name: String,
    start: Instant,
}

impl ElapsedTimeMeasure {
    pub fn new(name: &str) -> ElapsedTimeMeasure {
        ElapsedTimeMeasure {
            name: name.to_string(),
            start: Instant::now(),
        }
    }
}

impl Drop for ElapsedTimeMeasure {
    fn drop(&mut self) {
        let end = Instant::now();
        let d = end - self.start;
        //println!("{}: {}.{}", self.name, d.as_secs(), d.subsec_millis());
    }
}

pub struct ElapsedTimeMeasureCollector {
    instance_resolver_time: Duration,
}

impl ElapsedTimeMeasureCollector {
    pub fn new() -> ElapsedTimeMeasureCollector {
        ElapsedTimeMeasureCollector {
            instance_resolver_time: Duration::new(0, 0),
        }
    }

    pub fn add_instance_resolver_time(duration: Duration) {
        MEASUREMENT_COLLECTOR.with(|m| {
            let mut m = m.borrow_mut();
            m.instance_resolver_time += duration;
        });
    }

    pub fn print_instance_resolver_time() {
        MEASUREMENT_COLLECTOR.with(|m| {
            let m = m.borrow();
            /*
            println!(
                "InstanceResolver: {}.{}",
                m.instance_resolver_time.as_secs(),
                m.instance_resolver_time.subsec_millis()
            );
            */
        });
    }
}

thread_local! {
    static MEASUREMENT_COLLECTOR: RefCell<ElapsedTimeMeasureCollector> =
                                  RefCell::new(ElapsedTimeMeasureCollector::new());
}
