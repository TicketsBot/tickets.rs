use async_trait::async_trait;

use super::ShardManager;
use super::Options;

use crate::gateway::{Shard, Identify, ShardInfo};

use model::user::{StatusUpdate, ActivityType, StatusType};

use std::sync::Arc;

use std::collections::HashMap;

use cache::PostgresCache;
use darkredis::ConnectionPool;

use tokio::task::JoinHandle;
use crate::manager::FatalError;
use tokio::sync::mpsc;

pub struct PublicShardManager {
    join_handles: Vec<JoinHandle<()>>,
    error_rx: mpsc::Receiver<FatalError>,
}

impl PublicShardManager {
    pub fn connect(options: Options, cache: Arc<PostgresCache>, redis: Arc<ConnectionPool>) -> PublicShardManager {
        let mut shards = HashMap::new();

        let (error_tx, error_rx) = mpsc::channel(16);

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
                Option::<mpsc::Receiver<StatusUpdate>>::None,
            );

            shards.insert(i, shard);
        }

        let sm = PublicShardManager {
            join_handles: <PublicShardManager as ShardManager>::connect(shards),
            error_rx,
        };

        sm
    }
}

#[async_trait]
impl ShardManager for PublicShardManager {
    fn get_join_handles(&mut self) -> &mut Vec<JoinHandle<()>> {
        &mut self.join_handles
    }

    // TODO: Sentry
    async fn start_error_loop(&mut self) {
        while let Some(msg) = self.error_rx.recv().await {
            eprintln!("A fatal error occurred: {:?}", msg);
        }
    }
}
