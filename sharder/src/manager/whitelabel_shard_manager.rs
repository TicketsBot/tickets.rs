use async_trait::async_trait;

use super::ShardManager;

use crate::gateway::{Identify, Shard, ShardInfo};

use crate::gateway::event_forwarding::EventForwarder;
use crate::{Config, GatewayError};
use cache::PostgresCache;
use common::token_change;
use database::{Database, WhitelabelBot};
use deadpool_redis::Pool;
use futures::StreamExt;
use model::user::{ActivityType, StatusType, StatusUpdate};
use model::Snowflake;
use std::collections::HashMap;
use std::str;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;

pub struct WhitelabelShardManager<T: EventForwarder> {
    config: Arc<Config>,
    shards: RwLock<HashMap<Snowflake, Arc<Shard<T>>>>,
    // user_id -> bot_id
    user_ids: RwLock<HashMap<Snowflake, Snowflake>>,
    database: Arc<Database>,
    cache: Arc<PostgresCache>,
    redis: Arc<Pool>,
    event_forwarder: Arc<T>,
}

impl<T: EventForwarder> WhitelabelShardManager<T> {
    pub fn new(
        config: Config,
        database: Arc<Database>,
        cache: Arc<PostgresCache>,
        redis: Arc<Pool>,
        event_forwarder: Arc<T>,
    ) -> Self {
        WhitelabelShardManager {
            config: Arc::new(config),
            shards: RwLock::new(HashMap::new()),
            user_ids: RwLock::new(HashMap::new()),
            database,
            cache,
            redis,
            event_forwarder,
        }
    }

    async fn connect_bot(self: Arc<Self>, bot: WhitelabelBot) {
        self.user_ids
            .write()
            .await
            .insert(Snowflake(bot.user_id as u64), Snowflake(bot.bot_id as u64));

        let bot_id = Snowflake(bot.bot_id as u64);

        tokio::spawn(async move {
            // retrieve bot status
            let (status, status_type) = match self.database.whitelabel_status.get(bot_id).await {
                Ok((status, Some(status_type))) => Some((status, status_type)),
                Ok((status, None)) => {
                    eprintln!("Bot {} has invalid status type", bot_id);
                    None
                }
                Err(database::sqlx::Error::RowNotFound) => None,
                Err(e) => {
                    eprintln!(
                        "Error occurred while retrieving status for {}: {:?}",
                        bot.bot_id, e
                    );
                    None
                }
            }
            .unwrap_or_else(|| ("to /help".to_owned(), ActivityType::Listening));

            let shard_info = ShardInfo::new(0, 1);
            let presence = StatusUpdate::new(status_type, status, StatusType::Online);
            let identify = Identify::new(
                bot.token.clone(),
                None,
                shard_info,
                Some(presence),
                super::get_intents(),
            );

            let shard = Shard::new(
                self.config.clone(),
                identify,
                1,
                Arc::clone(&self.cache),
                Arc::clone(&self.redis),
                bot_id,
                Arc::clone(&self.event_forwarder),
                #[cfg(feature = "whitelabel")]
                Arc::clone(&self.database),
            );

            self.shards.write().await.insert(bot_id, Arc::clone(&shard));

            loop {
                let shard = Arc::clone(&shard);
                shard.log("Starting...");

                let res = Arc::clone(&shard).connect(None).await;
                match res {
                    Ok(()) => shard.log("Exited with Ok"),
                    Err(GatewayError::AuthenticationError { data, .. }) => {
                        shard.log_err(
                            "Exited with authentication error, removing ",
                            &GatewayError::custom(&data.error),
                        );

                        let bot_id = Snowflake(bot.bot_id as u64);
                        self.shards.write().await.remove(&bot_id);

                        let user_id = Snowflake(bot.user_id as u64);
                        self.user_ids.write().await.remove(&user_id);

                        if let Err(e) = self
                            .database
                            .whitelabel_errors
                            .append(Snowflake(bot.user_id as u64), data.error)
                            .await
                        {
                            shard.log_err(
                                "Error occurred while recording error to database",
                                &GatewayError::DatabaseError(e),
                            );
                        }

                        // TODO: Remove if statement
                        if data.status_code != 4014 {
                            self.delete_from_db(&bot.token).await;
                        }
                    }
                    Err(e) => shard.log_err("Exited with error", &e),
                }

                // we've received delete payload
                if self.shards.read().await.get(&bot_id).is_none() {
                    shard.log("Shard was removed from shard vec, not restarting");
                    break;
                } else {
                    shard.log("Shard still exists, restarting");
                }

                sleep(Duration::from_millis(500)).await;
            }
        });
    }

    async fn delete_from_db(&self, token: &str) {
        if let Err(e) = self.database.whitelabel.delete_by_token(token).await {
            eprintln!("Error removing bot: {}", e);
        }
    }

    pub async fn listen_status_updates(self: Arc<Self>) -> Result<(), GatewayError> {
        let database = Arc::clone(&self.database);

        let mut conn = redis::Client::open(self.config.get_redis_uri())
            .unwrap()
            .get_async_connection()
            .await?
            .into_pubsub();

        conn.subscribe(common::status_updates::KEY).await?;

        tokio::spawn(async move {
            let mut stream = conn.on_message();

            while let Some(m) = stream.next().await {
                match m.get_payload::<String>().map(|s| s.parse::<Snowflake>()) {
                    Ok(Ok(bot_id)) => {
                        if let Some(shard) = self.shards.read().await.get(&bot_id) {
                            shard.log("Received status update payload");

                            // retrieve new status
                            // TODO: New tokio::spawn for this?
                            match database.whitelabel_status.get(bot_id).await {
                                Ok((status, Some(status_type))) => {
                                    let tx = shard.status_update_tx.clone();
                                    let status =
                                        StatusUpdate::new(status_type, status, StatusType::Online);

                                    if let Err(e) = tx.send(status).await {
                                        eprintln!(
                                            "An error occured while updating status for {}: {}",
                                            bot_id, e
                                        );
                                    }
                                }

                                Ok((status, None)) => {
                                    eprintln!("Bot {} has invalid status type", bot_id);
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
        let mut conn = redis::Client::open(self.config.get_redis_uri())
            .unwrap()
            .get_async_connection()
            .await?
            .into_pubsub();

        conn.subscribe(common::token_change::KEY).await?;

        tokio::spawn(async move {
            let mut stream = conn.on_message();

            while let Some(m) = stream.next().await {
                let manager = Arc::clone(&self);

                match serde_json::from_slice::<token_change::Payload>(m.get_payload_bytes()) {
                    Ok(payload) => {
                        // check whether this shard has the old bot
                        if payload.old_id.0 % (manager.config.sharder_total as u64)
                            == manager.config.sharder_id as u64
                        {
                            if let Some(shard) = self.shards.write().await.remove(&payload.old_id) {
                                shard.log("Received token update payload, stopping");
                                shard.kill();
                            }
                        }

                        // start new bot
                        if payload.new_id.0 % (manager.config.sharder_total as u64)
                            == manager.config.sharder_id as u64
                        {
                            match self.database.whitelabel.get_bot_by_id(payload.new_id).await {
                                Ok(Some(bot)) => {
                                    manager.connect_bot(bot).await;
                                }
                                Ok(None) => {
                                    eprintln!("Couldn't find row for bot {}", payload.new_id)
                                }
                                Err(e) => eprintln!("Error retrieving bot from DB: {}", e),
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("An error occurred while decoding new token payload: {}", e)
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn listen_delete(self: Arc<Self>) -> Result<(), GatewayError> {
        let mut conn = redis::Client::open(self.config.get_redis_uri())
            .unwrap()
            .get_async_connection()
            .await?
            .into_pubsub();

        conn.subscribe(common::status_updates::KEY).await?;

        tokio::spawn(async move {
            let mut stream = conn.on_message();

            while let Some(m) = stream.next().await {
                match m.get_payload::<String>().map(|s| s.parse::<Snowflake>()) {
                    Ok(Ok(user_id)) => {
                        if let Some(bot_id) = self.user_ids.write().await.remove(&user_id) {
                            if let Some(shard) = self.shards.write().await.remove(&bot_id) {
                                shard.log("Received delete payload, stopping");
                                shard.kill();
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        eprintln!("An error occurred while reading delete payload: {}", e)
                    }
                    Err(e) => eprintln!("An error occurred while reading delete payload: {}", e),
                }
            }
        });

        Ok(())
    }
}

#[async_trait]
#[cfg(feature = "whitelabel")]
impl<T: EventForwarder> ShardManager for WhitelabelShardManager<T> {
    async fn connect(self: Arc<Self>) {
        // we should panic if we cant read db
        let bots = self
            .database
            .whitelabel
            .get_bots_by_sharder(self.config.sharder_total, self.config.sharder_id)
            .await
            .unwrap();

        for bot in bots {
            Arc::clone(&self).connect_bot(bot).await;
        }
    }
}
