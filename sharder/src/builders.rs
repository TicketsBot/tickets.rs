use crate::Config;
use cache::{Options, PostgresCache};
use deadpool::managed::PoolConfig;
use deadpool_redis::{Config as RedisConfig, Pool};

/// panics on err
pub async fn build_cache(config: &Config) -> PostgresCache {
    let cache_opts = Options {
        users: false,
        guilds: true,
        members: false,
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
    let cfg = RedisConfig {
        url: Some(config.get_redis_uri()),
        pool: Some(PoolConfig::new(config.redis_threads)),
    };

    cfg.create_pool().expect("Failed to create Redis pool")
}

pub fn setup_sentry(config: &Config) -> sentry::ClientInitGuard {
    // init sentry
    let guard = sentry::init((
        &config.sentry_dsn[..],
        sentry::ClientOptions {
            attach_stacktrace: true,
            ..Default::default()
        },
    ));

    let mut log_builder = env_logger::builder();
    log_builder.parse_filters("info");
    let logger = sentry_log::SentryLogger::with_dest(log_builder.build());

    log::set_boxed_logger(Box::new(logger)).unwrap();

    guard
}
