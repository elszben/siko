
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ItemPath {
    pub path: Vec<String>,
}

impl ItemPath {
    pub fn get(&self) -> String {
        let p: Vec<_> = self.path.iter().map(|i| i.clone()).collect();
        p.join(".")
    }
}
