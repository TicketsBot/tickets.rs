use model::Snowflake;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server_addr: Box<str>,
    pub public_bot_id: Snowflake,
    pub public_token: String,
    #[serde(with = "shim")]
    pub public_public_key: ed25519_dalek::PublicKey,

    pub database_uri: Box<str>,
    pub database_threads: u32,

    pub cache_uri: String,
    pub cache_threads: usize,

    pub worker_svc_uri: Box<str>,
    pub shard_count: u16,
}

// shim
mod shim {
    use ed25519_dalek::PublicKey;
    use serde::de::Error;
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<ed25519_dalek::PublicKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let key = String::deserialize(deserializer)?;

        let mut bytes = [0u8; 32];
        hex::decode_to_slice(key.as_bytes(), &mut bytes).unwrap();

        PublicKey::from_bytes(&bytes).map_err(D::Error::custom)
    }
}

impl Config {
    pub fn from_envvar() -> Config {
        envy::from_env().unwrap()
    }

    pub fn get_svc_uri(&self) -> Box<str> {
        format!("http://{}/interaction", self.worker_svc_uri).into_boxed_str()
    }
}
