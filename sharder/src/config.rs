use serde::Deserialize;

#[cfg(not(feature = "whitelabel"))]
use model::Snowflake;

#[derive(Deserialize, Debug)]
pub struct Config {
    // Required
    pub sharder_id: u16,
    pub sharder_total: u16,
    pub cache_uri: String,
    pub cache_threads: usize,
    pub redis_addr: String,
    pub redis_password: Option<String>,
    pub redis_threads: usize,
    pub worker_svc_uri: String,
    pub sentry_dsn: String,

    #[cfg(feature = "metrics")]
    pub metrics_addr: String,

    // Public Sharder
    #[cfg(not(feature = "whitelabel"))]
    pub large_sharding_buckets: u16,
    #[cfg(not(feature = "whitelabel"))]
    pub sharder_token: String,
    #[cfg(not(feature = "whitelabel"))]
    pub sharder_cluster_size: u16,
    #[cfg(not(feature = "whitelabel"))]
    pub bot_id: Snowflake,

    // Whitelabel Sharder
    pub database_uri: String,
    #[serde(default = "one")]
    pub database_threads: u32,
}

impl Config {
    pub fn from_envvar() -> Config {
        envy::from_env::<Config>().expect("Parsing config failed")
    }

    pub fn get_worker_svc_uri(&self) -> String {
        format!("http://{}/event", self.worker_svc_uri)
    }
    pub fn get_redis_uri(&self) -> String {
        match &self.redis_password {
            Some(pwd) => format!("redis://:{}@{}/", pwd, self.redis_addr),
            None => format!("redis://{}/", self.redis_addr),
        }
    }
}

fn one() -> u32 {
    return 1;
}
