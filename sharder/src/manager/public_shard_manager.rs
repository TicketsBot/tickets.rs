use async_trait::async_trait;

use super::ShardManager;
use super::Options;

use crate::gateway::{Shard, Identify, ShardInfo};

use model::user::{StatusUpdate, ActivityType, StatusType};

use std::sync::Arc;

use std::collections::HashMap;

use cache::PostgresCache;

use crate::manager::FatalError;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::delay_for;
use std::time::Duration;
use deadpool_redis::Pool;

pub struct PublicShardManager {
    shards: RwLock<HashMap<u16, Arc<Shard>>>,
    error_rx: Mutex<mpsc::Receiver<FatalError>>,
}

impl PublicShardManager {
    pub async fn new(options: Options, cache: Arc<PostgresCache>, redis: Arc<Pool>, ready_tx: mpsc::Sender<u16>) -> Arc<PublicShardManager> {
        let (error_tx, error_rx) = mpsc::channel(16);

        let sm = Arc::new(PublicShardManager {
            shards: RwLock::new(HashMap::new()),
            error_rx: Mutex::new(error_rx),
        });

        for i in options.shard_count.lowest..options.shard_count.highest {
            let shard_info = ShardInfo::new(i, options.shard_count.total);
            let status = StatusUpdate::new(ActivityType::Game, "DM for help | t!help".to_owned(), StatusType::Online);
            let identify = Identify::new(options.token.clone(), None, shard_info, Some(status), super::get_intents());
            let shard = Shard::new(
                identify,
                options.large_sharding_buckets,
                Arc::clone(&cache),
                Arc::clone(&redis),
                false,
                error_tx.clone(),
                Some(ready_tx.clone()),
            );

            sm.shards.write().await.insert(i, shard);
        }

        sm
    }
}

#[async_trait]
impl ShardManager for PublicShardManager {
    async fn connect(self: Arc<Self>) {
        for (_, shard) in self.shards.read().await.iter() {
            let shard = Arc::clone(&shard);

            tokio::spawn(async move {
                loop {
                    let shard = Arc::clone(&shard);
                    shard.log("Starting...");

                    match Arc::clone(&shard).connect().await {
                        Ok(()) => shard.log("Exited with Ok"),
                        Err(e) => shard.log_err("Exited with error", &e),
                    }

                    delay_for(Duration::from_millis(500)).await;
                }
            });
        }
    }

    // TODO: Sentry
    async fn start_error_loop(self: Arc<Self>) {
        while let Some(msg) = self.error_rx.lock().await.recv().await {
            eprintln!("A fatal error occurred: {:?}", msg);
        }
    }
}
