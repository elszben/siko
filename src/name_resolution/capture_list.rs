use crate::name_resolution::environment::NamedRef;

#[derive(Debug)]
pub struct CaptureList {
    captures: Vec<NamedRef>,
    level: usize,
}

impl CaptureList {
    pub fn new(level: usize) -> CaptureList {
        CaptureList {
            captures: Vec::new(),
            level: level,
        }
    }

    pub fn process(&mut self, r: NamedRef, level: usize) {
        if level < self.level {
            self.captures.push(r);
        }
    }

    pub fn captures(&self) -> Vec<NamedRef> {
        self.captures.clone()
    }
}
