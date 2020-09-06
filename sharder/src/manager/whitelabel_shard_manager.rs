// TODO Listen for new tokens
// TODO Listen for deletions

use async_trait::async_trait;

use super::ShardManager;

use crate::gateway::{Shard, Identify, ShardInfo};

use std::collections::HashMap;
use model::user::{StatusUpdate, ActivityType, StatusType};
use cache::PostgresCache;
use std::sync::Arc;
use database::{Database, WhitelabelBot};
use tokio::sync::{Mutex, mpsc, RwLock};
use model::Snowflake;
use tokio::task::JoinHandle;
use crate::manager::FatalError;
use crate::GatewayError;
use darkredis::ConnectionPool;
use std::str;
use tokio::stream::StreamExt;
use common::token_change;

pub struct WhitelabelShardManager {
    sharder_count: u16,
    sharder_id: u16,
    join_handles: Vec<JoinHandle<()>>,
    error_rx: mpsc::Receiver<FatalError>,
    db: Arc<Database>,
    redis: Arc<ConnectionPool>,
    status_update: Arc<RwLock<HashMap<Snowflake, mpsc::Sender<StatusUpdate>>>>, // bot_id -> tx
    kill_tx: Arc<RwLock<HashMap<Snowflake, mpsc::Sender<()>>>>,
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
        let kill_tx = Arc::new(RwLock::new(HashMap::new()));

        for bot in bots {
            Arc::clone(&)
        }

        futures::future::join_all(handles).await;

        let mu = match Arc::try_unwrap(shards) {
            Ok(m) => m,
            Err(e) => panic!(e),
        };

        let sm = WhitelabelShardManager {
            sharder_count,
            sharder_id,
            join_handles: <WhitelabelShardManager as ShardManager>::connect(mu.into_inner()),
            error_rx,
            db: Arc::clone(&database),
            redis: Arc::clone(&redis),
            status_update,
            kill_tx,
        };

        sm
    }

    async fn connect_bot(&self: Arc<WhitelabelShardManager>, bot: WhitelabelBot) {
        let bot_id = Snowflake(bot.bot_id as u64);

        let shards = Arc::clone(&self.shards);
        let status_update = Arc::clone(&self.status_update);
        let kill_tx = Arc::clone(&self.kill_tx);

        let database = Arc::clone(&self.database);
        let cache = Arc::clone(&self.cache);
        let redis = Arc::clone(&self.redis);
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

            kill_tx.write().await.insert(bot_id, shard.kill_shard_tx.clone());

            let mut shards = shards.lock().await;
            shards.insert(0, shard);
        }));
    }

    async fn delete_from_db(&self, token: &str) {
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

    pub async fn listen_new_tokens(&self) -> Result<(), GatewayError> {
        let sharder_count = self.sharder_count;
        let sharder_id = self.sharder_id;
        let kill_tx = Arc::clone(&self.kill_tx);

        let listener = self.redis.spawn(None).await.map_err(GatewayError::RedisError)?;
        let mut stream = listener.subscribe(&[token_change::KEY]).await.map_err(GatewayError::RedisError)?; // TODO: Move to common

        tokio::spawn(async move {
            while let Some(m) = stream.next().await {
                match serde_json::from_slice::<token_change::Payload>(&m.message[..]) {
                    Ok(payload) => {
                        // check whether this shard has the old bot
                        if payload.old_id.0 % (sharder_count as u64) == sharder_id as u64 {
                            if let Some(kill_tx) = kill_tx.read().await.get(&payload.old_id) {
                                if let Err(e) = kill_tx.clone().send(()).await {
                                    eprintln!("An error occurred while killing {}: {}", payload.old_id, e);
                                }
                            }
                        }

                        // start new bot
                        if payload.new_id.0 % (sharder_count as u64) == sharder_id as u64 {

                        }
                    }
                    Err(e) => eprintln!("An error occurred while decoding new token payload: {}", e),
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

            self.delete_from_db(&msg.bot_token).await;
        }
    }
}
