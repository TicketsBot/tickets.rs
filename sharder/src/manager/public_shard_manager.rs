use async_trait::async_trait;

use super::ShardManager;
use super::Options;

use crate::gateway::{Shard, Identify, ShardInfo};

use model::user::{StatusUpdate, ActivityType, StatusType};

use std::sync::Arc;
use tokio::time::delay_for;

use std::collections::HashMap;
use std::time::Duration;

use cache::PostgresCache;

use r2d2_redis::RedisConnectionManager;
use r2d2_redis::r2d2::Pool;
use tokio::task::JoinHandle;

pub struct PublicShardManager {
    join_handles: Vec<JoinHandle<()>>,
}

impl PublicShardManager {
    pub fn connect(options: Options, cache: Arc<PostgresCache>, redis: Arc<Pool<RedisConnectionManager>>) -> PublicShardManager {
        let mut shards = HashMap::new();

        for i in options.shard_count.lowest..options.shard_count.highest {
            let shard_info = ShardInfo::new(i, options.shard_count.total);
            let status = StatusUpdate::new(ActivityType::Game, "DM for help | t!help".to_owned(), StatusType::Online);
            let identify = Identify::new(options.token.clone(), None, shard_info, Some(status), super::get_intents());
            let shard = Shard::new(identify, options.large_sharding_buckets, Arc::clone(&cache), Arc::clone(&redis), false);

            shards.insert(i, shard);
        }

        let sm = PublicShardManager {
            join_handles: <PublicShardManager as ShardManager>::connect(shards),
        };

        sm
    }
}

#[async_trait]
impl ShardManager for PublicShardManager {
    fn connect(shards: HashMap<u16, Shard>) -> Vec<JoinHandle<()>> {
        let mut handles = Vec::with_capacity(shards.len());
        for (i, mut shard) in shards {
            handles.push(tokio::spawn(async move {
                loop {
                    println!("Starting shard {}", i);

                    match shard.connect().await {
                        Ok(()) => println!("Shard {} exited with Ok", i),
                        Err(e) => eprintln!("Shard {} exited with err: {:?}", i, e)
                    }

                    delay_for(Duration::from_millis(500)).await;
                }
            }));
        }

        handles
    }

    fn is_whitelabel() -> bool {
        false
    }

    fn get_join_handles(&mut self) -> &mut Vec<JoinHandle<()>> {
        &mut self.join_handles
    }
}
