use async_trait::async_trait;

use super::ShardManager;

use crate::gateway::payloads::Identify;
use crate::gateway::{Shard, ShardInfo};

use crate::gateway::event_forwarding::EventForwarder;
use crate::{
    Config, GatewayError, InternalCommand, RedisSessionStore, SessionData, SessionStore,
    ShardIdentifier,
};
use common::token_change;
use database::{Database, WhitelabelBot};
use deadpool_redis::redis;
use deadpool_redis::Pool;
use futures::StreamExt;
use model::user::{ActivityType, StatusType, StatusUpdate};
use model::Snowflake;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{sleep, timeout};
use tracing::{error, info, warn};

pub struct WhitelabelShardManager<T: EventForwarder> {
    config: Arc<Config>,
    database: Arc<Database>,
    redis: Arc<Pool>,
    session_store: RedisSessionStore,
    event_forwarder: Arc<T>,
    shutdown_tx: broadcast::Sender<mpsc::Sender<(ShardIdentifier, Option<SessionData>)>>,
    shard_command_channels: RwLock<HashMap<Snowflake, mpsc::Sender<InternalCommand>>>,
}

impl<T: EventForwarder> WhitelabelShardManager<T> {
    pub fn new(
        config: Config,
        database: Arc<Database>,
        redis: Arc<Pool>,
        session_store: RedisSessionStore,
        event_forwarder: Arc<T>,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        WhitelabelShardManager {
            config: Arc::new(config),
            database,
            redis,
            session_store,
            event_forwarder,
            shutdown_tx,
            shard_command_channels: RwLock::new(HashMap::new()),
        }
    }

    async fn connect_bot(self: Arc<Self>, bot: WhitelabelBot) {
        let bot_id = Snowflake(bot.bot_id as u64);

        let sm = Arc::clone(&self);
        tokio::spawn(async move {
            // retrieve bot status
            let (status, status_type) = match self.database.whitelabel_status.get(bot_id).await {
                Ok((status, Some(status_type))) => Some((status, status_type)),
                Ok((_, None)) => {
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

            let mut resume_data = sm
                .session_store
                .get(bot_id.0)
                .await
                .expect("Failed to fetch session data"); // TODO: Log, not panic

            loop {
                let shard_info = ShardInfo::new(0, 1);
                let presence = StatusUpdate::new(status_type, status.clone(), StatusType::Online);
                let identify = Identify::new(
                    bot.token.clone(),
                    None,
                    shard_info,
                    Some(presence),
                    super::get_intents(),
                );

                let (command_tx, command_rx) = mpsc::channel(4);

                let shard = Shard::new(
                    self.config.clone(),
                    identify,
                    1,
                    Arc::clone(&self.redis),
                    bot_id,
                    Arc::clone(&self.event_forwarder),
                    None,
                    self.shutdown_tx.subscribe(),
                    Arc::clone(&self.database),
                    command_rx,
                );

                // Validate bot still exists
                match sm.database.whitelabel.get_bot_by_id(bot_id).await {
                    Ok(Some(_)) => (),
                    Ok(None) => {
                        info!("Bot no longer exists, stopping");
                        break;
                    }
                    Err(e) => {
                        error!(error = %e, "Error checking if bot still exists in the database");
                        sleep(Duration::from_secs(10)).await;
                        continue;
                    }
                }

                info!("Starting");

                {
                    sm.shard_command_channels
                        .write()
                        .await
                        .insert(bot_id, command_tx);
                }

                let res = shard.connect(resume_data.clone()).await;

                {
                    sm.shard_command_channels.write().await.remove(&bot_id);
                }

                match res {
                    Ok(session_data) => {
                        resume_data = session_data;
                        self.log_for_bot(bot_id, "Exited with Ok");
                    }
                    Err(GatewayError::AuthenticationError { data, .. }) => {
                        self.log_err_for_bot(
                            bot_id,
                            "Exited with authentication error, removing bot",
                            &GatewayError::custom(&data.error),
                        );

                        if let Err(e) = self
                            .database
                            .whitelabel_errors
                            .append(Snowflake(bot.user_id as u64), data.error)
                            .await
                        {
                            self.log_err_for_bot(
                                bot_id,
                                "Error occurred while recording error to database",
                                &GatewayError::DatabaseError(e),
                            );
                        }

                        if let Err(e) = self.database.whitelabel.delete_by_bot_id(bot_id).await {
                            self.log_err_for_bot(
                                bot_id,
                                "Error occurred while deleting bot",
                                &GatewayError::DatabaseError(e),
                            );
                        }

                        break;
                    }
                    Err(e) => self.log_err_for_bot(bot_id, "Exited with error", &e),
                }

                sleep(Duration::from_millis(500)).await;
            }
        });
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
                        if let Some(tx) = self.shard_command_channels.read().await.get(&bot_id) {
                            println!("[RPC] Received status update payload for bot {bot_id}");

                            // retrieve new status
                            // TODO: New tokio::spawn for this?
                            match database.whitelabel_status.get(bot_id).await {
                                Ok((status, Some(status_type))) => {
                                    let status =
                                        StatusUpdate::new(status_type, status, StatusType::Online);

                                    let cmd = InternalCommand::StatusUpdate { status };

                                    if let Err(e) = tx.send(cmd).await {
                                        eprintln!(
                                            "An error occured while updating status for {}: {}",
                                            bot_id, e
                                        );
                                    }
                                }

                                Ok((_, None)) => {
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

        conn.subscribe("tickets:whitelabeldelete").await?;

        let sm = Arc::clone(&self);
        tokio::spawn(async move {
            let mut stream = conn.on_message();

            while let Some(m) = stream.next().await {
                match m.get_payload::<String>().map(|s| s.parse::<Snowflake>()) {
                    Ok(Ok(bot_id)) => {
                        println!("[RPC] Received delete payload for {bot_id}");

                        let channels = sm.shard_command_channels.read().await;
                        let command_tx = channels.get(&bot_id);
                        if let Some(command_tx) = command_tx {
                            if let Err(e) = command_tx.send(InternalCommand::Shutdown).await {
                                eprintln!(
                                    "An error occurred while sending shutdown command for {}: {}",
                                    bot_id, e
                                );
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

    pub fn log_for_bot(&self, bot_id: Snowflake, msg: impl Display) {
        info!(bot_id = %bot_id, "{msg}");
    }

    pub fn log_err_for_bot(&self, bot_id: Snowflake, msg: impl Display, err: &GatewayError) {
        error!(bot_id = %bot_id, error = %err, "{msg}");
    }
}

#[async_trait]
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

    async fn shutdown(self: Arc<Self>) {
        let (tx, mut rx) = mpsc::channel(self.shutdown_tx.receiver_count());

        let receivers = self
            .shutdown_tx
            .send(tx)
            .expect("Failed to send shutdown signal to shards");

        let mut sessions = HashMap::new();
        for _ in 0..receivers {
            let (identifier, session_data) = match timeout(Duration::from_secs(30), rx.recv()).await
            {
                Ok(Some((identifier, Some(session_data)))) => (identifier, session_data),
                Ok(Some((identifier, None))) => {
                    let bot_id = identifier.bot_id;
                    info!(bot_id = %bot_id, "Shard sent None session data");
                    continue;
                }
                Ok(None) => {
                    warn!("Shutdown session data receiver is closed");
                    break;
                }
                Err(e) => {
                    warn!(error = %e, "Timeout while waiting for shard to shutdown");
                    break;
                }
            };

            sessions.insert(identifier.bot_id.0, session_data);
        }

        if let Err(e) = self.session_store.set_bulk(sessions).await {
            error!(error = %e, "Failed to save session data");
        }
    }
}
