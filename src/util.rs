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
    format!("[{}]", ss.join(", "))
}

pub fn format_list_simple<T: fmt::Display>(items: &[T]) -> String {
    let ss: Vec<_> = items.iter().map(|i| format!("{}", i)).collect();
    format!("{}", ss.join(", "))
}
