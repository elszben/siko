pub struct Config {
    pub measure_durations: bool,
    pub visualize: bool,
}

impl Config {
    pub fn new() -> Config {
        Config {
            measure_durations: false,
            visualize: false,
        }
    }
}
