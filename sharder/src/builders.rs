use super::var_or_panic;

use cache::{PostgresCache, Options};
use sqlx::postgres::PgPoolOptions;
use darkredis::ConnectionPool;
use std::env;

/// panics on err
pub async fn build_cache() -> PostgresCache {
    let cache_uri = &var_or_panic("CACHE_URI");
    let cache_opts = Options {
        users: true,
        guilds: true,
        members: true,
        channels: true,
        roles: true,
        emojis: false,
        voice_states: false,
    };

    let pg_opts = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(var_or_panic("CACHE_THREADS").parse().unwrap());

    PostgresCache::connect(cache_uri, cache_opts, pg_opts).await.unwrap()
}

/// panics on err
pub async fn build_redis() -> ConnectionPool {
    ConnectionPool::create(
        var_or_panic("REDIS_ADDR"),
        env::var("REDIS_PASSWORD").ok().as_ref().map(|s| s.as_str()),
        var_or_panic("REDIS_THREADS").parse().unwrap()
    ).await.unwrap()
}