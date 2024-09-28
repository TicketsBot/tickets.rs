use serde::Deserialize;

#[cfg(not(feature = "whitelabel"))]
use model::Snowflake;

#[derive(Deserialize, Debug)]
pub struct Config {
    // Required
    pub sharder_id: u16,
    pub sharder_total: u16,
    pub redis_addr: String,
    pub redis_password: Option<String>,
    pub redis_threads: usize,
    pub sentry_dsn: String,
    pub worker_svc_uri: Option<String>,
    pub kafka_brokers: Vec<String>,
    pub kafka_topic: String,

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
    #[cfg(feature = "whitelabel")]
    pub database_uri: String,
    #[cfg(feature = "whitelabel")]
    #[serde(default = "one")]
    pub database_threads: u32,
}

impl Config {
    pub fn from_envvar() -> Config {
        envy::from_env::<Config>().expect("Parsing config failed")
    }

    pub fn get_worker_svc_uri(&self) -> Option<String> {
        self.worker_svc_uri
            .clone()
            .map(|s| format!("http://{}/event", s))
    }

    pub fn get_redis_uri(&self) -> String {
        match &self.redis_password {
            Some(pwd) => format!("redis://:{}@{}/", pwd, self.redis_addr),
            None => format!("redis://{}/", self.redis_addr),
        }
    }
}

#[cfg(feature = "whitelabel")]
fn one() -> u32 {
    1
}
