use std::sync::Arc;

use sharder::{PublicShardManager, ShardCount, ShardManager};
use model::user::{StatusUpdate, ActivityType, StatusType};

use sharder::{var_or_panic, build_cache, build_redis};

use jemallocator::Jemalloc;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

/*#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;*/

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // init sharder options
    let presence = StatusUpdate::new(ActivityType::Listening, "t!help".to_owned(), StatusType::Online);
    let options = sharder::Options {
        token: var_or_panic("SHARDER_TOKEN"),
        shard_count: get_shard_count(),
        presence,
        large_sharding_buckets: 1,
    };

    // init cache
    let cache = Arc::new(build_cache().await);

    // init redis
    let redis = Arc::new(build_redis());

    let sm = PublicShardManager::new(options, cache, redis).await;
    Arc::clone(&sm).connect().await;

    sm.start_error_loop().await;
    Ok(())
}

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

