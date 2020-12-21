use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::fs::File;
use tokio::signal;

use sharder::{PublicShardManager, ShardCount, ShardManager, Config};
use model::user::{StatusUpdate, ActivityType, StatusType};

use sharder::{var_or_panic, build_cache, build_redis};

use jemallocator::Jemalloc;
use model::Snowflake;
use std::str::FromStr;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() {
    // init sharder options
    let config = Arc::new(Config::from_envvar());
    let shard_count = get_shard_count();

    let ready_tx = handle_ready_probe((shard_count.highest - shard_count.lowest) as usize);

    let presence = StatusUpdate::new(ActivityType::Listening, "t!help".to_owned(), StatusType::Online);
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

    let sm = PublicShardManager::new(config, options, cache, redis, ready_tx).await;
    Arc::new(sm).connect().await;

    signal::ctrl_c().await.expect("Failed to listen for ctrl_c");
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

fn handle_ready_probe(shard_count: usize) -> mpsc::Sender<u16> {
    let (tx, mut rx) = mpsc::channel(shard_count);

    tokio::spawn(async move {
        let mut ready = 0;

        while let Some(shard_id) = rx.recv().await {
            println!("[{:0>2}] Loaded guilds", shard_id);
            ready += 1;

            if ready == shard_count {
                File::create("/tmp/ready").await.unwrap(); // panic if can't create
                println!("Reported readiness to probe");
                break
            }
        }
    });

    tx
}

