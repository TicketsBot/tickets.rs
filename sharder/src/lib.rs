mod gateway;
pub use gateway::*;

mod manager;
pub use manager::{Options, ShardCount, ShardManager};

#[cfg(not(feature = "whitelabel"))]
pub use manager::PublicShardManager;

#[cfg(feature = "whitelabel")]
pub use manager::WhitelabelShardManager;

mod builders;
pub use builders::{build_cache, build_redis, get_redis_uri, get_worker_svc_uri};

mod config;
pub use config::Config;

use std::env;

pub fn var_or_panic(s: &str) -> Box<str> {
    let var = env::var(s).expect(&format!("Couldn't parse envvar {}", s)[..]);

    match var.strip_suffix("\r") {
        Some(s) => Box::from(s),
        None => var.into_boxed_str(),
    }
}

pub fn var(s: &str) -> Option<String> {
    let var = match env::var(s) {
        Ok(var) => var,
        Err(_) => return None,
    };

    let var = match var.strip_suffix("\r") {
        Some(s) => s.to_owned(),
        None => var,
    };

    Some(var)
}
