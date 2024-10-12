use crate::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub workers: usize,
    pub brokers: Vec<String>,
    pub group_id: String,
    pub topic: String,
    pub postgres_uri: String,
    pub metric_server_addr: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        envy::from_env().map_err(Into::into)
    }
}
