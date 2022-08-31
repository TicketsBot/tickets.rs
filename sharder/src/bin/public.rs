use std::sync::Arc;
use tokio::signal;

use model::user::{ActivityType, StatusType, StatusUpdate};
use sharder::{setup_sentry, Config, PublicShardManager, ShardCount, ShardManager};

use sharder::{build_cache, build_redis};

use deadpool_redis::redis::cmd;
use jemallocator::Jemalloc;
use sharder::event_forwarding::HttpEventForwarder;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

/*#[cfg(feature = "whitelabel")]
fn main() {
    panic!("Started public sharder with whitelabel feature flag")
}

#[cfg(not(feature = "whitelabel"))]*/
#[tokio::main]
async fn main() {
    // init sharder options
    let config = Config::from_envvar();

    #[cfg(feature = "use-sentry")]
    let _guard = setup_sentry(&config);

    #[cfg(not(feature = "use-sentry"))]
    env_logger::init();

    let shard_count = get_shard_count(&config);

    let options = sharder::Options {
        token: Box::from(config.sharder_token.clone()),
        shard_count,
        presence: StatusUpdate::new(
            ActivityType::Listening,
            "to /help".to_owned(),
            StatusType::Online,
        ),
        large_sharding_buckets: 1,
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

    let event_forwarder = Arc::new(HttpEventForwarder::new(
        HttpEventForwarder::build_http_client(),
    ));

    let sm = PublicShardManager::new(config, options, cache, redis, event_forwarder).await;
    Arc::new(sm).connect().await;

    signal::ctrl_c().await.expect("Failed to listen for ctrl_c");
}

#[cfg(not(feature = "whitelabel"))]
fn get_shard_count(config: &Config) -> ShardCount {
    ShardCount {
        total: config.sharder_cluster_size * config.sharder_total,
        lowest: config.sharder_cluster_size * config.sharder_id,
        highest: config.sharder_cluster_size * (config.sharder_id + 1),
    }
}
