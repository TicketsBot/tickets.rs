use super::var_or_panic;

use std::env;

use cache::{PostgresCache, Options};
use sqlx::postgres::PgPoolOptions;
use r2d2_redis::r2d2::Pool;
use r2d2_redis::RedisConnectionManager;
use r2d2_redis::redis::{ConnectionInfo, ConnectionAddr};

/// panics on err
pub async fn build_cache() -> PostgresCache {
    let cache_uri = &var_or_panic("CACHE_URI");
    let cache_opts = Options {
        users: false,
        guilds: false,
        members: false,
        channels: false,
        roles: false,
        emojis: false,
        voice_states: false,
    };

    let pg_opts = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(var_or_panic("CACHE_THREADS").parse().unwrap());

    PostgresCache::connect(cache_uri, cache_opts, pg_opts).await.unwrap()
}

/// panics on err
pub async fn build_redis() -> Pool<RedisConnectionManager> {
    let pwd = match env::var("REDIS_PASSWD") {
        Ok(pwd) => Some(pwd),
        Err(_) => None,
    };

    let conn_addr = ConnectionAddr::Tcp(
        var_or_panic("REDIS_ADDR"),
        env::var("REDIS_PORT").map(|p| p.parse().unwrap_or(6379)).unwrap_or(6379)
    );

    let connection_info = ConnectionInfo {
        addr: Box::new(conn_addr),
        db: 0,
        passwd: pwd,
    };

    let manager = RedisConnectionManager::new(connection_info).unwrap();
    r2d2::Pool::builder()
        .max_size(var_or_panic("REDIS_THREADS").parse().unwrap())
        .build(manager)
        .unwrap()
}