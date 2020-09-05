// TODO Listen for new tokens
// TODO Listen for deletions
// TODO Listen for status updates

use async_trait::async_trait;

use super::ShardManager;

use crate::gateway::{Shard, Identify, ShardInfo};

use std::collections::HashMap;
use model::user::{StatusUpdate, ActivityType, StatusType};
use cache::PostgresCache;
use r2d2_redis::RedisConnectionManager;
use r2d2_redis::r2d2::Pool;
use std::sync::Arc;
use database::Database;
use tokio::sync::{Mutex, mpsc};
use model::Snowflake;
use tokio::task::JoinHandle;
use crate::manager::FatalError;

pub struct WhitelabelShardManager {
    join_handles: Vec<JoinHandle<()>>,
    error_rx: mpsc::Receiver<FatalError>,
    db: Arc<Database>,
}

impl WhitelabelShardManager {
    pub async fn connect(
        sharder_count: u16,
        sharder_id: u16,
        database: Arc<Database>,
        cache: Arc<PostgresCache>,
        redis: Arc<Pool<RedisConnectionManager>>
    ) -> WhitelabelShardManager {
        // we should panic if we cant read db
        let bots = database.whitelabel.get_bots_by_sharder(sharder_count, sharder_id).await.unwrap();

        let (error_tx, error_rx) = mpsc::channel(16);

        let shards: Arc<Mutex<HashMap<u16, Shard>>> = Arc::new(Mutex::new(HashMap::new()));
        let mut handles = Vec::with_capacity(bots.len());
        for bot in bots {
            let shards = Arc::clone(&shards);

            let database = Arc::clone(&database);
            let cache = Arc::clone(&cache);
            let redis = Arc::clone(&redis);
            let error_tx = error_tx.clone();

            handles.push(tokio::spawn(async move {
                // retrieve bot status
                let status = match database.whitelabel_status.get(Snowflake(bot.bot_id as u64)).await {
                    Ok(status) => Some(status),
                    Err(sqlx::Error::RowNotFound) => None,
                    Err(e) => {
                        eprintln!("Error occurred while retrieving status for {}: {:?}", bot.bot_id, e);
                        None
                    },
                }.unwrap_or("for t!help".to_owned());

                let shard_info = ShardInfo::new(0, 1);
                let presence = StatusUpdate::new(ActivityType::Listening, status, StatusType::Online);
                let identify = Identify::new(bot.token, None, shard_info, Some(presence), super::get_intents());

                let shard = Shard::new(identify, 1, cache, redis, true, error_tx);

                let mut shards = shards.lock().await;
                shards.insert(0, shard);
            }));
        }

        futures::future::join_all(handles).await;

        let mu = match Arc::try_unwrap(shards) {
            Ok(m) => m,
            Err(e) => panic!(e),
        };

        let sm = WhitelabelShardManager {
            join_handles: <WhitelabelShardManager as ShardManager>::connect(mu.into_inner()),
            error_rx,
            db: Arc::clone(&database),
        };

        sm
    }

    async fn remove_bot(&self, token: &str) {
        if let Err(e) = self.db.whitelabel.delete_by_token(token).await {
            eprintln!("Error removing bot: {}", e);
        }
    }
}

#[async_trait]
impl ShardManager for WhitelabelShardManager {
    fn get_join_handles(&mut self) -> &mut Vec<JoinHandle<()>> {
        &mut self.join_handles
    }

    // TODO: Sentry?
    async fn start_error_loop(&mut self) {
        while let Some(msg) = self.error_rx.recv().await {
            eprintln!("A bot received a fatal error, removing: {:?}", msg);

            // get bot ID
            match self.db.whitelabel.get_bot_by_token(&msg.bot_token).await {
                Ok(Some(bot)) => {
                    if let Err(e) = self.db.whitelabel_errors.append(Snowflake(bot.user_id as u64), msg.error).await {
                        eprintln!("Error while logging error: {}", e);
                    }
                },
                Ok(None) => {
                    eprintln!("Bot had no ID, removing anyway");
                },
                Err(e) => {
                    eprintln!("Error getting bot: {}", e);
                }
            }

            self.remove_bot(&msg.bot_token).await;
        }
    }
}
