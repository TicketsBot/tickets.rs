use ed25519_dalek::PublicKey;
use model::Snowflake;
use std::env;

pub struct Config {
    pub server_addr: Box<str>,
    pub main_bot_id: Snowflake,
    pub main_bot_token: Box<str>,
    pub main_public_key: ed25519_dalek::PublicKey,
    pub database: DatabaseConfig,
    #[cfg(feature = "sticky-cookie")]
    pub worker_sticky_cookie: Box<str>,
    pub worker_svc_uri: Box<str>,
    pub shard_count: u16,
}

pub struct DatabaseConfig {
    pub uri: Box<str>,
    pub threads: u32,
}

impl Config {
    pub fn from_envvar() -> Config {
        Config {
            server_addr: Config::get_envvar("SERVER_ADDR"),
            main_bot_id: Snowflake(Config::get_envvar("PUBLIC_BOT_ID").parse().unwrap()),
            main_bot_token: Config::get_envvar("PUBLIC_TOKEN"),
            main_public_key: Config::read_public_key(),
            database: DatabaseConfig {
                uri: Config::get_envvar("DATABASE_URI"),
                threads: Config::get_envvar("DATABASE_THREADS").parse().unwrap(),
            },
            #[cfg(feature = "sticky-cookie")]
            worker_sticky_cookie: Config::get_envvar("WORKER_STICKY_COOKIE"),
            worker_svc_uri: Config::get_svc_uri(),
            shard_count: Config::get_envvar("SHARD_COUNT").parse().unwrap(),
        }
    }

    pub fn get_envvar(name: &str) -> Box<str> {
        let var = env::var(name).expect(&format!("envvar {} was missing!", name)[..]);

        match var.strip_suffix("\r") {
            Some(s) => Box::from(s),
            None => var.into_boxed_str(),
        }
    }

    pub fn get_envvar_or_none(name: &str) -> Option<String> {
        let var = match env::var(name) {
            Ok(var) => var,
            Err(_) => return None,
        };

        let var = match var.strip_suffix("\r") {
            Some(s) => s.to_owned(),
            None => var,
        };

        Some(var)
    }

    fn read_public_key() -> ed25519_dalek::PublicKey {
        let key = Config::get_envvar("PUBLIC_PUBLIC_KEY");

        let mut bytes = [0u8; 32];
        hex::decode_to_slice(key.as_bytes(), &mut bytes).unwrap();

        PublicKey::from_bytes(&bytes).unwrap()
    }

    fn get_svc_uri() -> Box<str> {
        format!(
            "http://{}/interaction",
            Config::get_envvar("WORKER_SVC_URI")
        )
        .into_boxed_str()
    }
}
