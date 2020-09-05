mod gateway;
pub use gateway::*;

mod manager;
pub use manager::{ShardManager, PublicShardManager, WhitelabelShardManager, Options, ShardCount};

mod builders;
pub use builders::{build_cache, build_redis};

use std::env;
pub fn var_or_panic(s: &str) -> String {
    env::var(s).unwrap()
}