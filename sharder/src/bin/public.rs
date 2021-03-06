use std::sync::Arc;
use tokio::signal;

use model::user::{ActivityType, StatusType, StatusUpdate};
use sharder::{Config, PublicShardManager, ShardCount, ShardManager};

use sharder::{build_cache, build_redis, var_or_panic};

use deadpool_redis::cmd;
use jemallocator::Jemalloc;
use model::Snowflake;
use sharder::event_forwarding::HttpEventForwarder;
use std::str::FromStr;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(feature = "whitelabel")]
fn main() {
    panic!("Started public sharder with whitelabel feature flag")
}

#[cfg(not(feature = "whitelabel"))]
#[tokio::main]
async fn main() {
    // init sharder options
    let config = Arc::new(Config::from_envvar());
    let shard_count = get_shard_count();

    let presence = StatusUpdate::new(
        ActivityType::Listening,
        "/help".to_owned(),
        StatusType::Online,
    );
    let options = sharder::Options {
        token: var_or_panic("SHARDER_TOKEN"),
        shard_count,
        presence,
        large_sharding_buckets: 1,
        user_id: Snowflake::from_str(&var_or_panic("BOT_ID")[..]).unwrap(),
    };

    // init cache
    let cache = Arc::new(build_cache().await);
    //cache.create_schema().await.unwrap();

    // init redis
    let redis = Arc::new(build_redis());

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
    Arc::clone(&event_forwarder).start_reset_cookie_loop();

    let sm = PublicShardManager::new(config, options, cache, redis, event_forwarder).await;
    Arc::new(sm).connect().await;

    signal::ctrl_c().await.expect("Failed to listen for ctrl_c");
}

#[cfg(not(feature = "whitelabel"))]
fn get_shard_count() -> ShardCount {
    let cluster_size: u16 = var_or_panic("SHARDER_CLUSTER_SIZE").parse().unwrap();
    let sharder_count: u16 = var_or_panic("SHARDER_TOTAL").parse().unwrap();
    let sharder_id: u16 = var_or_panic("SHARDER_ID").parse().unwrap();

    ShardCount {
        total: cluster_size * sharder_count,
        lowest: cluster_size * sharder_id,
        highest: cluster_size * (sharder_id + 1),
    }
}
