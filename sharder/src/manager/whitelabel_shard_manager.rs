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
use crate::manager::FatalError;
use crate::GatewayError;
use darkredis::ConnectionPool;
use std::str;
use tokio::stream::StreamExt;
use common::token_change;
use tokio::time::delay_for;
use std::time::Duration;

pub struct WhitelabelShardManager {
    sharder_count: u16,
    sharder_id: u16,
    shards: RwLock<HashMap<Snowflake, Arc<Shard>>>,
    // user_id -> bot_id
    user_ids: RwLock<HashMap<Snowflake, Snowflake>>,
    error_rx: Mutex<mpsc::Receiver<FatalError>>,
    error_tx: mpsc::Sender<FatalError>,
    db: Arc<Database>,
    cache: Arc<PostgresCache>,
    redis: Arc<ConnectionPool>,
}

impl WhitelabelShardManager {
    pub async fn new(
        sharder_count: u16,
        sharder_id: u16,
        database: Arc<Database>,
        cache: Arc<PostgresCache>,
        redis: Arc<ConnectionPool>,
    ) -> Arc<Self> {
        let (error_tx, error_rx) = mpsc::channel(16);

        let sm = Arc::new(WhitelabelShardManager {
            sharder_count,
            sharder_id,
            shards: RwLock::new(HashMap::new()),
            user_ids: RwLock::new(HashMap::new()),
            error_rx: Mutex::new(error_rx),
            error_tx,
            db: Arc::clone(&database),
            cache,
            redis,
        });

        sm
    }

    async fn connect_bot(self: Arc<Self>, bot: WhitelabelBot) {
        self.user_ids.write().await.insert(Snowflake(bot.user_id as u64), Snowflake(bot.bot_id as u64));

        let bot_id = Snowflake(bot.bot_id as u64);

        let error_tx = self.error_tx.clone();

        tokio::spawn(async move {
            // retrieve bot status
            let status = match self.db.whitelabel_status.get(bot_id).await {
                Ok(status) => Some(status),
                Err(sqlx::Error::RowNotFound) => None,
                Err(e) => {
                    eprintln!("Error occurred while retrieving status for {}: {:?}", bot.bot_id, e);
                    None
                }
            }.unwrap_or("t!help".to_owned());

            let shard_info = ShardInfo::new(0, 1);
            let presence = StatusUpdate::new(ActivityType::Listening, status, StatusType::Online);
            let identify = Identify::new(bot.token, None, shard_info, Some(presence), super::get_intents());

            let shard = Shard::new(
                identify,
                1,
                Arc::clone(&self.cache),
                Arc::clone(&self.redis),
                true,
                error_tx,
            );

            self.shards.write().await.insert(bot_id, Arc::clone(&shard));

            loop {
                let shard = Arc::clone(&shard);
                shard.log("Starting...");

                match Arc::clone(&shard).connect().await {
                    Ok(()) => shard.log("Exited with Ok"),
                    Err(e) => shard.log_err("Exited with error", &e),
                }

                // we've received delete payload
                if self.shards.read().await.get(&bot_id).is_none() {
                    break;
                }

                delay_for(Duration::from_millis(500)).await;
            }
        });
    }

    async fn delete_from_db(&self, token: &str) {
        if let Err(e) = self.db.whitelabel.delete_by_token(token).await {
            eprintln!("Error removing bot: {}", e);
        }
    }

    pub async fn listen_status_updates(self: Arc<Self>) -> Result<(), GatewayError> {
        let database = Arc::clone(&self.db);

        let listener = self.redis.spawn(None).await.map_err(GatewayError::RedisError)?;
        let mut stream = listener.subscribe(&[common::status_updates::KEY]).await.map_err(GatewayError::RedisError)?;

        tokio::spawn(async move {
            while let Some(m) = stream.next().await {
                match str::from_utf8(&m.message[..]).map(|s| s.parse::<Snowflake>()) {
                    Ok(Ok(bot_id)) => {
                        if let Some(shard) = self.shards.read().await.get(&bot_id) {
                            // retrieve new status
                            // TODO: New tokio::spawn for this?
                            match database.whitelabel_status.get(bot_id).await {
                                Ok(status) => {
                                    if let Err(e) = shard.status_update_tx.clone().send(StatusUpdate::new(ActivityType::Listening, status, StatusType::Online)).await {
                                        eprintln!("An error occured while updating status for {}: {}", bot_id, e);
                                    }
                                }
                                Err(e) => eprintln!("Error retrieving status from db: {}", e),
                            }
                        }
                    }
                    Ok(Err(e)) => eprintln!("An error occured while reading status updates: {}", e),
                    Err(e) => eprintln!("An error occured while reading status updates: {}", e),
                }
            }
        });

        Ok(())
    }

    pub async fn listen_new_tokens(self: Arc<Self>) -> Result<(), GatewayError> {
        let listener = self.redis.spawn(None).await.map_err(GatewayError::RedisError)?;
        let mut stream = listener.subscribe(&[token_change::KEY]).await.map_err(GatewayError::RedisError)?;

        tokio::spawn(async move {
            while let Some(m) = stream.next().await {
                let manager = Arc::clone(&self);

                match serde_json::from_slice::<token_change::Payload>(&m.message[..]) {
                    Ok(payload) => {
                        // check whether this shard has the old bot
                        if payload.old_id.0 % (manager.sharder_count as u64) == manager.sharder_id as u64 {
                            if let Some(shard) = self.shards.read().await.get(&payload.old_id) {
                                self.shards.write().await.remove(&payload.old_id);
                                if let Err(e) = shard.kill_shard_tx.clone().send(()).await {
                                    eprintln!("An error occurred while killing {}: {}", payload.old_id, e);
                                }
                            }
                        }

                        // start new bot
                        if payload.new_id.0 % (manager.sharder_count as u64) == manager.sharder_id as u64 {
                            match self.db.whitelabel.get_bot_by_id(payload.new_id).await {
                                Ok(Some(bot)) => {
                                    manager.connect_bot(bot).await;
                                }
                                Ok(None) => eprintln!("Couldn't find row for bot {}", payload.new_id),
                                Err(e) => eprintln!("Error retrieving bot from DB: {}", e),
                            }
                        }
                    }
                    Err(e) => eprintln!("An error occurred while decoding new token payload: {}", e),
                }
            }
        });

        Ok(())
    }

    pub async fn listen_delete(self: Arc<Self>) -> Result<(), GatewayError> {
        let listener = self.redis.spawn(None).await.map_err(GatewayError::RedisError)?;
        let mut stream = listener.subscribe(&["tickets:whitelabeldelete"]).await.map_err(GatewayError::RedisError)?; // TODO: Move to common

        tokio::spawn(async move {
            while let Some(m) = stream.next().await {
                match str::from_utf8(&m.message[..]).map(|s| s.parse::<Snowflake>()) {
                    Ok(Ok(user_id)) => {
                        // get bot for user
                        let mut user_ids = self.user_ids.write().await;
                        let mut shards = self.shards.write().await;

                        if let Some(bot_id) = user_ids.get(&user_id) {
                            if let Some(shard) = shards.get(bot_id) {
                                shard.kill().await;
                            }

                            shards.remove(&bot_id);
                        }

                        user_ids.remove(&user_id);
                    }
                    Ok(Err(e)) => eprintln!("An error occured while reading delete payload: {}", e),
                    Err(e) => eprintln!("An error occured while reading delete payload: {}", e),
                }
            }
        });

        Ok(())
    }
}

#[async_trait]
impl ShardManager for WhitelabelShardManager {
    async fn connect(self: Arc<Self>) {
        // we should panic if we cant read db
        let bots = self.db.whitelabel.get_bots_by_sharder(self.sharder_count, self.sharder_id).await.unwrap();

        for bot in bots {
            Arc::clone(&self).connect_bot(bot).await;
        }
    }

    // TODO: Sentry?
    async fn start_error_loop(self: Arc<Self>) {
        while let Some(msg) = self.error_rx.lock().await.recv().await {
            eprintln!("A bot received a fatal error, removing: {:?}", msg);

            // get bot ID
            match self.db.whitelabel.get_bot_by_token(&msg.bot_token).await {
                Ok(Some(bot)) => {
                    if let Err(e) = self.db.whitelabel_errors.append(Snowflake(bot.user_id as u64), msg.error).await {
                        eprintln!("Error while logging error: {}", e);
                    }
                }
                Ok(None) => {
                    eprintln!("Bot had no ID, removing anyway");
                }
                Err(e) => {
                    eprintln!("Error getting bot: {}", e);
                }
            }

            self.delete_from_db(&msg.bot_token).await;
        }
    }
}
