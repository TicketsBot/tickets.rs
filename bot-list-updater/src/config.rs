use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub delay: u64,
    pub bot_id: u64,
    pub base_url: String,

    pub dbl_token: String,
    pub dboats_token: String,
}

impl Config {
    pub fn from_envvar() -> Config {
        envy::from_env().unwrap()
    }
}
