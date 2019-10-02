pub struct Config {
    pub verbose: bool,
    pub visualize: bool,
}

impl Config {
    pub fn new() -> Config {
        Config {
            verbose: false,
            visualize: false,
        }
    }
}
