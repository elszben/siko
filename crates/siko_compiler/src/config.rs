pub struct Config {
    pub verbose: bool,
}

impl Config {
    pub fn new() -> Config {
        Config { verbose: false }
    }
}
