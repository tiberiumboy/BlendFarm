pub struct Config {
    pub host: String,
    pub port: u16,
}

impl Config {
    pub fn load() -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: 15000,
        }
    }
}
