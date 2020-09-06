// TODO Listen for new tokens
// TODO Listen for deletions

use async_trait::async_trait;

use super::ShardManager;

use crate::gateway::{Shard, Identify, ShardInfo};

use std::collections::HashMap;
use model::user::{StatusUpdate, ActivityType, StatusType};
use cache::PostgresCache;
use std::sync::Arc;
use database::Database;
use tokio::sync::{Mutex, mpsc, RwLock};
use model::Snowflake;
use tokio::task::JoinHandle;
use crate::manager::FatalError;
use crate::GatewayError;
use darkredis::ConnectionPool;
use std::str;
use tokio::stream::StreamExt;

pub struct WhitelabelShardManager {
    join_handles: Vec<JoinHandle<()>>,
    error_rx: mpsc::Receiver<FatalError>,
    db: Arc<Database>,
    redis: Arc<ConnectionPool>,
    status_update: Arc<RwLock<HashMap<Snowflake, mpsc::Sender<StatusUpdate>>>>, // bot_id -> tx
}

impl WhitelabelShardManager {
    pub async fn connect(
        sharder_count: u16,
        sharder_id: u16,
        database: Arc<Database>,
        cache: Arc<PostgresCache>,
        redis: Arc<ConnectionPool>
    ) -> WhitelabelShardManager {
        // we should panic if we cant read db
        let bots = database.whitelabel.get_bots_by_sharder(sharder_count, sharder_id).await.unwrap();

        let (error_tx, error_rx) = mpsc::channel(16);

        let shards: Arc<Mutex<HashMap<u16, Shard>>> = Arc::new(Mutex::new(HashMap::new()));

        let mut handles = Vec::with_capacity(bots.len());
        let status_update = Arc::new(RwLock::new(HashMap::new()));

        for bot in bots {
            let bot_id = Snowflake(bot.bot_id as u64);

            let shards = Arc::clone(&shards);
            let status_update = Arc::clone(&status_update);

            let database = Arc::clone(&database);
            let cache = Arc::clone(&cache);
            let redis = Arc::clone(&redis);
            let error_tx = error_tx.clone();

            handles.push(tokio::spawn(async move {
                // retrieve bot status
                let status = match database.whitelabel_status.get(bot_id).await {
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

                let (status_update_tx, status_update_rx) = mpsc::channel(1);
                status_update.write().await.insert(bot_id, status_update_tx);

                let shard = Shard::new(
                    identify,
                    1,
                    cache,
                    redis,
                    true,
                    error_tx,
                    Some(status_update_rx)
                );

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
            redis: Arc::clone(&redis),
            status_update
        };

        sm
    }

    async fn remove_bot(&self, token: &str) {
        if let Err(e) = self.db.whitelabel.delete_by_token(token).await {
            eprintln!("Error removing bot: {}", e);
        }
    }

    pub async fn listen_status_updates(&self) -> Result<(), GatewayError> {
        let database = Arc::clone(&self.db);
        let status_update = Arc::clone(&self.status_update);

        let listener = self.redis.spawn(None).await.map_err(GatewayError::RedisError)?;
        let mut stream = listener.subscribe(&[common::status_updates::KEY]).await.map_err(GatewayError::RedisError)?;

        tokio::spawn(async move {
            while let Some(m) = stream.next().await {
                let database = Arc::clone(&database);
                let status_update = Arc::clone(&status_update);
                match str::from_utf8(&m.message[..]).map(|s| s.parse::<Snowflake>()) {
                    Ok(Ok(bot_id)) => {
                        if let Some(tx) = (*status_update.read().await).get(&bot_id) {
                            // retrieve new status
                            // TODO: New tokio::spawn for this?
                            match database.whitelabel_status.get(bot_id).await {
                                Ok(status) => {
                                    if let Err(e) = tx.clone().send(StatusUpdate::new(ActivityType::Listening, status, StatusType::Online)).await {
                                        eprintln!("An error occured while updating status for {}: {}", bot_id, e);
                                    }
                                },
                                Err(e) => eprintln!("Error retrieving status from db: {}", e),
                            }
                        }
                    },
                    Ok(Err(e)) => eprintln!("An error occured while reading status updates: {}", e),
                    Err(e) => eprintln!("An error occured while reading status updates: {}", e),
                }
            }
        });

        Ok(())
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
