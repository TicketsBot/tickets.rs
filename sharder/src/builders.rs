use super::var_or_panic;

use cache::{PostgresCache, Options};
use deadpool_redis::{Config, Pool};
use deadpool::managed::PoolConfig;
use crate::var;

/// panics on err
pub async fn build_cache() -> PostgresCache {
    let cache_uri = &var_or_panic("CACHE_URI");
    let cache_opts = Options {
        users: false,
        guilds: true,
        members: false,
        channels: true,
        roles: true,
        emojis: false,
        voice_states: false,
    };

    let cache_threads = var_or_panic("CACHE_THREADS").parse::<usize>().unwrap();

    /*let pg_opts = PgPoolOptions::new()
        .min_connections(cache_threads as u32)
        .max_connections(cache_threads as u32);*/

    PostgresCache::connect(cache_uri, cache_opts, cache_threads).await.unwrap()
}

/// panics on err
pub fn build_redis() -> Pool {
    let mut cfg = Config::default();
    cfg.url = Some(get_redis_uri());
    cfg.pool = Some(PoolConfig::new(var_or_panic("REDIS_THREADS").parse().unwrap()));

    cfg.create_pool().unwrap()
}

pub fn get_redis_uri() -> String {
    let addr = var_or_panic("REDIS_ADDR");

    match var("REDIS_PASSWORD") {
        Some(pwd) => format!("redis://:{}@{}/", pwd, addr),
        None => format!("redis://{}/", addr),
    }
}

pub fn get_worker_svc_uri() -> Box<str> {
    let uri = var_or_panic("WORKER_SVC_URI");
    format!("http://{}/event", uri).into_boxed_str()
}
