use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server_addr: String,
    pub cache_uri: String,
}

impl Config {
    pub fn new() -> Config {
        envy::from_env().expect("failed to parse config")
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
