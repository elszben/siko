use std::fmt;

pub struct Indent {
    indent: usize,
}

impl Indent {
    pub fn new() -> Indent {
        Indent { indent: 0 }
    }

    pub fn inc(&mut self) {
        self.indent += 4;
    }

    pub fn dec(&mut self) {
        self.indent -= 4;
    }
}

impl fmt::Display for Indent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for _ in 0..self.indent {
            write!(f, " ")?
        }
        Ok(())
    }
}

pub fn get_module_name(name: &str) -> String {
    name.replace(".", "_")
}

pub fn arg_name(index: usize) -> String {
    format!("arg{}", index)
}
