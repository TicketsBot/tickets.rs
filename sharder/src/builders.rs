use crate::Config;
use cache::{Options, PostgresCache};
use deadpool::managed::PoolConfig;
use deadpool::Runtime;
use deadpool_redis::{Config as RedisConfig, Pool};

/// panics on err
pub async fn build_cache(config: &Config) -> PostgresCache {
    let cache_opts = Options {
        users: true,
        guilds: true,
        members: true,
        channels: true,
        roles: true,
        emojis: false,
        voice_states: false,
    };

    PostgresCache::connect(config.cache_uri.clone(), cache_opts, config.cache_threads)
        .await
        .unwrap()
}

/// panics on err
pub fn build_redis(config: &Config) -> Pool {
    let mut cfg = RedisConfig::from_url(config.get_redis_uri());
    cfg.pool = Some(PoolConfig::new(config.redis_threads));

    cfg.create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create Redis pool")
}

pub fn setup_sentry(config: &Config) -> sentry::ClientInitGuard {
    let mut log_builder = env_logger::builder();
    //log_builder.parse_filters("info");
    let logger = sentry_log::SentryLogger::with_dest(log_builder.build());

    log::set_boxed_logger(Box::new(logger)).expect("Failed to set logger");
    log::set_max_level(log::LevelFilter::Info);

    sentry::init((
        &config.sentry_dsn[..],
        sentry::ClientOptions {
            attach_stacktrace: true,
            ..Default::default()
        },
    ))
}
