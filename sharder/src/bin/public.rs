use std::sync::Arc;

use model::user::{ActivityType, StatusType, StatusUpdate};
use sharder::{
    await_shutdown, setup_sentry, Config, PublicShardManager, RedisSessionStore, ShardCount,
    ShardManager,
};

use sharder::{build_cache, build_redis, metrics_server};

use deadpool_redis::redis::cmd;
use jemallocator::Jemalloc;
use sharder::event_forwarding::HttpEventForwarder;
use tracing::info;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() {
    // init sharder options
    let config = Config::from_envvar();

    #[cfg(feature = "use-sentry")]
    let _guard = setup_sentry(&config);

    #[cfg(not(feature = "use-sentry"))]
    tracing_subscriber::fmt::init();

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

    // init cache
    let cache = Arc::new(build_cache(&config).await);

    // init redis
    let redis = Arc::new(build_redis(&config));

    // test redis connection
    let mut conn = redis.get().await.expect("Failed to get redis conn");

    let res: String = cmd("PING")
        .query_async(&mut conn)
        .await
        .expect("Redis PING failed");

    assert_eq!(res, "PONG");

    let session_store =
        RedisSessionStore::new(Arc::clone(&redis), "tickets:resume:public".to_string(), 300);

    let event_forwarder = Arc::new(HttpEventForwarder::default());

    let sm = PublicShardManager::new(
        config,
        options,
        session_store,
        cache,
        redis,
        event_forwarder,
    )
    .await;

    let sm = Arc::new(sm);
    Arc::clone(&sm).connect().await;

    await_shutdown()
        .await
        .expect("Failed to wait for shutdown signal");
    info!("Received shutdown signal");

    sm.shutdown().await;
    info!("Shard manager shutdown gracefully");
}

#[cfg(not(feature = "whitelabel"))]
fn get_shard_count(config: &Config) -> ShardCount {
    ShardCount {
        total: config.sharder_cluster_size * config.sharder_total,
        lowest: config.sharder_cluster_size * config.sharder_id,
        highest: config.sharder_cluster_size * (config.sharder_id + 1),
    }
}
