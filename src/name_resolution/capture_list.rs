#[derive(Debug)]
pub struct CaptureList {
    captures: Vec<String>,
    level: usize,
}

impl CaptureList {
    pub fn new(level: usize) -> CaptureList {
        CaptureList {
            captures: Vec::new(),
            level: level,
        }
    }

    pub fn process(&mut self, var: &str, level: usize) {
        if level < self.level {
            self.captures.push(var.to_string());
        }
    }

    pub fn captures(&self) -> Vec<String> {
        self.captures.clone()
    }
}
