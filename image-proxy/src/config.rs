use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server_addr: String,
    pub jwt_key: String,
}

impl Config {
    pub fn from_envvar() -> Config {
        envy::from_env().unwrap()
    }
}
