use async_trait::async_trait;

use super::ShardManager;

use crate::gateway::{Shard, Identify, ShardInfo};

use std::collections::HashMap;
use model::user::{StatusUpdate, ActivityType, StatusType, User};
use cache::PostgresCache;
use std::sync::Arc;
use database::{Database, WhitelabelBot};
use tokio::sync::RwLock;
use model::Snowflake;
use crate::{GatewayError, get_redis_uri};
use std::str;
use tokio::stream::StreamExt;
use common::token_change;
use tokio::time::delay_for;
use std::time::Duration;
use deadpool_redis::Pool;

pub struct WhitelabelShardManager {
    sharder_count: u16,
    sharder_id: u16,
    shards: RwLock<HashMap<Snowflake, Arc<Shard>>>,
    // user_id -> bot_id
    user_ids: RwLock<HashMap<Snowflake, Snowflake>>,
    db: Arc<Database>,
    cache: Arc<PostgresCache>,
    redis: Arc<Pool>,
}

impl WhitelabelShardManager {
    pub async fn new(
        sharder_count: u16,
        sharder_id: u16,
        database: Arc<Database>,
        cache: Arc<PostgresCache>,
        redis: Arc<Pool>,
    ) -> Arc<Self> {
        let sm = Arc::new(WhitelabelShardManager {
            sharder_count,
            sharder_id,
            shards: RwLock::new(HashMap::new()),
            user_ids: RwLock::new(HashMap::new()),
            db: Arc::clone(&database),
            cache,
            redis,
        });

        sm
    }

    async fn connect_bot(self: Arc<Self>, bot: WhitelabelBot) {
        self.user_ids.write().await.insert(Snowflake(bot.user_id as u64), Snowflake(bot.bot_id as u64));

        let bot_id = Snowflake(bot.bot_id as u64);

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
            let identify = Identify::new(bot.token.clone(), None, shard_info, Some(presence), super::get_intents());

            let shard = Shard::new(
                identify,
                1,
                Arc::clone(&self.cache),
                Arc::clone(&self.redis),
                true,
                None,
            );

            *shard.user.write().await = Some(User::blank(bot_id));

            self.shards.write().await.insert(bot_id, Arc::clone(&shard));

            loop {
                let shard = Arc::clone(&shard);
                shard.log("Starting...").await;

                match Arc::clone(&shard).connect().await {
                    Ok(()) => shard.log("Exited with Ok").await,
                    Err(e) => {
                        shard.log_err("Exited with error, quitting", &e).await;

                        let user_id = Snowflake(bot.user_id as u64);
                        self.shards.write().await.remove(&user_id);
                        self.user_ids.write().await.remove(&user_id);

                        let error_message = match e {
                            GatewayError::AuthenticationError { error, .. } => error,
                            _ => format!("{}", e),
                        };

                        if let Err(e) = self.db.whitelabel_errors.append(Snowflake(bot.user_id as u64), error_message).await {
                            eprintln!("Error while logging error: {}", e);
                        }

                        self.delete_from_db(&bot.token).await;
                        break;
                    }
                }

                // we've received delete payload
                if self.shards.read().await.get(&bot_id).is_none() {
                    shard.log("Shard was removed from shard vec, not restarting").await;
                    break;
                } else {
                    shard.log("Shard still exists, restarting").await;
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

        let mut conn = redis::Client::open(get_redis_uri()).unwrap().get_async_connection().await?.into_pubsub();
        conn.subscribe(common::status_updates::KEY).await.map_err(GatewayError::RedisError)?;

        tokio::spawn(async move {
            let mut stream = conn.on_message();

            while let Some(m) = stream.next().await {
                match m.get_payload::<String>().map(|s| s.parse::<Snowflake>()) {
                    Ok(Ok(bot_id)) => {
                        if let Some(shard) = self.shards.read().await.get(&bot_id) {
                            shard.log("Received status update payload").await;

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
        let mut conn = redis::Client::open(get_redis_uri()).unwrap().get_async_connection().await?.into_pubsub();
        conn.subscribe(common::token_change::KEY).await.map_err(GatewayError::RedisError)?;

        tokio::spawn(async move {
            let mut stream = conn.on_message();

            while let Some(m) = stream.next().await {
                let manager = Arc::clone(&self);

                match serde_json::from_slice::<token_change::Payload>(m.get_payload_bytes()) {
                    Ok(payload) => {
                        // check whether this shard has the old bot
                        if payload.old_id.0 % (manager.sharder_count as u64) == manager.sharder_id as u64 {
                            if let Some(shard) = self.shards.read().await.get(&payload.old_id) {
                                shard.log("Received token update payload, stopping").await;

                                self.shards.write().await.remove(&payload.old_id);
                                Arc::clone(&shard).kill();
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
        let mut conn = redis::Client::open(get_redis_uri()).unwrap().get_async_connection().await?.into_pubsub();
        conn.subscribe(common::status_updates::KEY).await.map_err(GatewayError::RedisError)?;

        tokio::spawn(async move {
            let mut stream = conn.on_message();

            while let Some(m) = stream.next().await {
                match m.get_payload::<String>().map(|s| s.parse::<Snowflake>()) {
                    Ok(Ok(user_id)) => {
                        if let Some(bot_id) = self.user_ids.write().await.remove(&user_id) {
                            if let Some(shard) = self.shards.write().await.remove(&bot_id) {
                                shard.log("Received delete payload, stopping").await;
                                shard.kill();
                            }
                        }
                    }
                    Ok(Err(e)) => eprintln!("An error occurred while reading delete payload: {}", e),
                    Err(e) => eprintln!("An error occurred while reading delete payload: {}", e),
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
}
