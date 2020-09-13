mod gateway;
pub use gateway::*;

mod manager;
pub use manager::{ShardManager, PublicShardManager, WhitelabelShardManager, Options, ShardCount};

mod builders;
pub use builders::{build_cache, build_redis, get_redis_uri};

use std::env;
pub fn var_or_panic(s: &str) -> String {
    let var = env::var(s).unwrap();

    match var.strip_suffix("\r") {
        Some(s) => s.to_owned(),
        None => var,
    }
}