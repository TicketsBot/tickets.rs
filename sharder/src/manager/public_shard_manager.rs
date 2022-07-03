use async_trait::async_trait;

use super::Options;
use super::ShardManager;

use crate::gateway::{Identify, Shard, ShardInfo};

use std::sync::Arc;

use std::collections::HashMap;

use cache::PostgresCache;

use crate::config::Config;
use crate::gateway::event_forwarding::EventForwarder;
use crate::GatewayError;
use deadpool_redis::Pool;
use std::time::Duration;
use tokio::fs::File;
use tokio::sync::oneshot;
use tokio::time::sleep;

pub struct PublicShardManager<T: EventForwarder> {
    config: Arc<Config>,
    shards: HashMap<u16, Arc<Shard<T>>>,
}

impl<T: EventForwarder> PublicShardManager<T> {
    pub async fn new(
        config: Config,
        options: Options,
        cache: Arc<PostgresCache>,
        redis: Arc<Pool>,
        event_forwarder: Arc<T>,
    ) -> Self {
        let mut sm = PublicShardManager {
            config: Arc::new(config),
            shards: HashMap::new(),
        };

        for i in options.shard_count.lowest..options.shard_count.highest {
            let shard_info = ShardInfo::new(i, options.shard_count.total);

            let identify = Identify::new(
                options.token.clone().into_string(),
                None,
                shard_info,
                Some(options.presence.clone()),
                super::get_intents(),
            );

            let shard = Shard::new(
                Arc::clone(&sm.config),
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
            let shard = Arc::clone(shard);
            let (ready_tx, ready_rx) = oneshot::channel::<()>();

            tokio::spawn(async move {
                let mut ready_tx = Some(ready_tx);

                loop {
                    let shard = Arc::clone(&shard);
                    shard.log("Starting...");

                    // TODO: Skip ready_rx await on error
                    match Arc::clone(&shard).connect(ready_tx.take()).await {
                        Ok(()) => shard.log("Exited with Ok"),
                        Err(GatewayError::AuthenticationError { data, .. }) => {
                            if data.should_reconnect() {
                                shard.log_err(
                                    "Authentication error, reconnecting",
                                    &GatewayError::custom(&data.error),
                                );
                            } else {
                                shard.log_err(
                                    "Fatal authentication error, shutting down",
                                    &GatewayError::custom(&data.error),
                                );
                                break;
                            }
                        }
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
