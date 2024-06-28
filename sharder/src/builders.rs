use crate::Config;
use cache::{Options, PostgresCache};
use deadpool::managed::PoolConfig;
use deadpool::Runtime;
use deadpool_redis::{Config as RedisConfig, Pool};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

/// panics on err
#[tracing::instrument(skip(config))]
pub async fn build_cache(config: &Config) -> PostgresCache {
    let cache_opts = Options {
        users: true,
        guilds: true,
        members: true,
        channels: true,
        threads: false,
        roles: false,
        emojis: false,
        voice_states: false,
    };

    PostgresCache::connect(config.cache_uri.clone(), cache_opts, config.cache_threads)
        .await
        .unwrap()
}

/// panics on err
#[tracing::instrument(skip(config))]
pub fn build_redis(config: &Config) -> Pool {
    let mut cfg = RedisConfig::from_url(config.get_redis_uri());
    cfg.pool = Some(PoolConfig::new(config.redis_threads));

    cfg.create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create Redis pool")
}

#[tracing::instrument(skip(config))]
pub fn setup_sentry(config: &Config) -> sentry::ClientInitGuard {
    let guard = sentry::init((
        &config.sentry_dsn[..],
        sentry::ClientOptions {
            release: sentry::release_name!(),
            attach_stacktrace: true,
            sample_rate: 1.0,
            traces_sample_rate: 0.1,
            ..Default::default()
        },
    ));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(sentry_tracing::layer())
        .with(EnvFilter::from_default_env())
        .init();

    guard
}
