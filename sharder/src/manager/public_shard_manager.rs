use async_trait::async_trait;

use super::Options;
use super::ShardManager;

use crate::gateway::{Identify, Shard, ShardInfo};

use model::user::{ActivityType, StatusType, StatusUpdate};

use std::sync::Arc;

use std::collections::HashMap;

use cache::PostgresCache;

use crate::config::Config;
use crate::gateway::event_forwarding::EventForwarder;
use deadpool_redis::Pool;
use std::time::Duration;
use tokio::fs::File;
use tokio::sync::oneshot;
use tokio::time::sleep;

pub struct PublicShardManager<T: EventForwarder> {
    config: Arc<Config>,
    shards: HashMap<u16, Arc<Shard<T>>>,
}

#[cfg(not(feature = "whitelabel"))]
impl<T: EventForwarder> PublicShardManager<T> {
    pub async fn new(
        config: Arc<Config>,
        options: Options,
        cache: Arc<PostgresCache>,
        redis: Arc<Pool>,
        event_forwarder: Arc<T>,
    ) -> Self {
        let mut sm = PublicShardManager {
            config,
            shards: HashMap::new(),
        };

        for i in options.shard_count.lowest..options.shard_count.highest {
            let shard_info = ShardInfo::new(i, options.shard_count.total);
            let status = StatusUpdate::new(
                ActivityType::Listening,
                "/help | /setup".to_owned(),
                StatusType::Online,
            );
            let identify = Identify::new(
                options.token.clone().into_string(),
                None,
                shard_info,
                Some(status),
                super::get_intents(),
            );

            let shard = Shard::new(
                sm.config.clone(),
                identify,
                options.large_sharding_buckets,
                Arc::clone(&cache),
                Arc::clone(&redis),
                options.user_id,
                Arc::clone(&event_forwarder),
            );

            sm.shards.insert(i, shard);
        }

        sm
    }
}

#[async_trait]
impl<T: EventForwarder> ShardManager for PublicShardManager<T> {
    async fn connect(self: Arc<Self>) {
        for (_, shard) in self.shards.iter() {
            let shard_id = shard.get_shard_id();
            let shard = Arc::clone(&shard);
            let (ready_tx, ready_rx) = oneshot::channel::<()>();

            tokio::spawn(async move {
                let mut ready_tx = Some(ready_tx);

                loop {
                    let shard = Arc::clone(&shard);
                    shard.log("Starting...");

                    match Arc::clone(&shard).connect(ready_tx.take()).await {
                        Ok(()) => shard.log("Exited with Ok"),
                        Err(e) => shard.log_err("Exited with error", &e),
                    }

                    sleep(Duration::from_millis(500)).await;
                }
            });

            match ready_rx.await {
                Ok(_) => println!("[{:0>2}] Loaded guilds", shard_id),
                Err(e) => eprintln!("[{:0>2}] Error reading ready rx: {}", shard_id, e),
            }
        }

        File::create("/tmp/ready").await.unwrap(); // panic if can't create
        println!("Reported readiness to probe");
    }
}
