use std::sync::Arc;

use database::sqlx::postgres::PgPoolOptions;
use database::Database;
use model::user::{ActivityType, StatusType, StatusUpdate};
use sharder::{
    await_shutdown, setup_sentry, Config, PublicShardManager, RedisSessionStore, ShardCount,
    ShardManager,
};

use sharder::{build_cache, build_redis, metrics_server, Result};

use deadpool_redis::redis::cmd;
use jemallocator::Jemalloc;
use sharder::event_forwarding::HttpEventForwarder;
use tracing::info;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

// Sentry doesn't support #[tokio::main]
fn main() -> Result<()> {
    // init sharder options
    let config = Config::from_envvar();

    #[cfg(feature = "use-sentry")]
    let _guard = setup_sentry(&config);

    #[cfg(not(feature = "use-sentry"))]
    tracing_subscriber::fmt::init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async { run(config).await })
}

#[tracing::instrument(skip(config))]
async fn run(config: Config) -> Result<()> {
    #[cfg(feature = "metrics")]
    {
        let metrics_addr = config.metrics_addr.clone();

        tokio::spawn(async move {
            metrics_server::start_server(metrics_addr.as_str())
                .await
                .expect("Failed to start metrics server");
        });
    }

    let shard_count = get_shard_count(&config);

    let options = sharder::Options {
        token: Box::from(config.sharder_token.clone()),
        shard_count,
        presence: StatusUpdate::new(
            ActivityType::Listening,
            "/help".to_owned(),
            StatusType::Online,
        ),
        large_sharding_buckets: config.large_sharding_buckets,
        user_id: config.bot_id,
    };

    // init db
    info!(service="database", connections = %config.database_threads, "Connecting to database");
    let db_opts = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(config.database_threads);
    let database = Arc::new(Database::connect(&config.database_uri[..], db_opts).await?);
    info!(service = "database", "Database connected");

    // init cache
    info!(
        service = "cache",
        threads = config.cache_threads,
        "Connecting to cache"
    );
    let cache = Arc::new(build_cache(&config).await);
    info!(service = "cache", "Cache connected");

    // init redis
    info!(
        service = "redis",
        threads = config.redis_threads,
        "Connecting to redis"
    );
    let redis = Arc::new(build_redis(&config));
    info!(service = "redis", "Redis connected");

    // test redis connection
    info!(service = "redis", "Testing redis connection");
    let mut conn = redis.get().await.expect("Failed to get redis conn");

    let res: String = cmd("PING")
        .query_async(&mut conn)
        .await
        .expect("Redis PING failed");

    assert_eq!(res, "PONG");
    info!(service = "redis", "Redis connection test successful");

    let session_store =
        RedisSessionStore::new(Arc::clone(&redis), "tickets:resume:public".to_string(), 300);

    let event_forwarder = Arc::new(HttpEventForwarder::default());

    let sm = PublicShardManager::new(
        config,
        options,
        session_store,
        database,
        cache,
        redis,
        event_forwarder,
    )
    .await;

    info!("Starting shard manager");
    let sm = Arc::new(sm);
    Arc::clone(&sm).connect().await;

    await_shutdown()
        .await
        .expect("Failed to wait for shutdown signal");
    info!("Received shutdown signal");

    sm.shutdown().await;
    info!("Shard manager shutdown gracefully");

    Ok(())
}

#[cfg(not(feature = "whitelabel"))]
fn get_shard_count(config: &Config) -> ShardCount {
    ShardCount {
        total: config.sharder_cluster_size * config.sharder_total,
        lowest: config.sharder_cluster_size * config.sharder_id,
        highest: config.sharder_cluster_size * (config.sharder_id + 1),
    }
}
