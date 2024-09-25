use async_trait::async_trait;
use tracing::debug;
use tracing::{error, info, warn};

use super::Options;
use super::ShardManager;

use crate::gateway::{payloads::Identify, Shard, ShardInfo};
use crate::{RedisSessionStore, SessionData, SessionStore, ShardIdentifier};

use std::collections::HashMap;
use std::sync::Arc;

use crate::config::Config;
use crate::gateway::event_forwarding::EventForwarder;
use crate::GatewayError;
use deadpool_redis::Pool;
use std::time::Duration;
use tokio::fs::File;
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::time::{sleep, timeout};

pub struct PublicShardManager<T: EventForwarder> {
    config: Arc<Config>,
    options: Options,
    session_store: RedisSessionStore,
    redis: Arc<Pool>,
    event_forwarder: Arc<T>,
    shutdown_tx: broadcast::Sender<mpsc::Sender<(ShardIdentifier, Option<SessionData>)>>,
}

impl<T: EventForwarder> PublicShardManager<T> {
    pub async fn new(
        config: Config,
        options: Options,
        session_store: RedisSessionStore,
        redis: Arc<Pool>,
        event_forwarder: Arc<T>,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            config: Arc::new(config),
            options,
            session_store,
            redis,
            event_forwarder,
            shutdown_tx,
        }
    }

    fn build_shard(&self, shard_id: u16, ready_tx: Option<oneshot::Sender<()>>) -> Shard<T> {
        let shard_info = ShardInfo::new(shard_id, self.options.shard_count.total);

        let identify = Identify::new(
            self.options.token.clone().into_string(),
            None,
            shard_info,
            Some(self.options.presence.clone()),
            super::get_intents(),
        );

        Shard::new(
            Arc::clone(&self.config),
            identify,
            self.options.large_sharding_buckets,
            Arc::clone(&self.redis),
            self.options.user_id,
            Arc::clone(&self.event_forwarder),
            ready_tx,
            self.shutdown_tx.subscribe(),
            #[cfg(feature = "whitelabel")]
            command_rx,
        )
    }

    #[tracing::instrument(skip(self, resume_data, ready_tx))]
    async fn start_shard(
        &self,
        shard_id: u16,
        resume_data: Option<SessionData>,
        ready_tx: Option<oneshot::Sender<()>>,
    ) -> (bool, Option<SessionData>) {
        let shard = self.build_shard(shard_id, ready_tx);

        // TODO: Skip ready_rx await on error
        match shard.connect(resume_data.clone()).await {
            Ok(session_data) => {
                info!("Shard exited normally");
                return (true, session_data);
            }
            Err(GatewayError::AuthenticationError { data, .. }) => {
                if data.should_reconnect() {
                    warn!(error = ?data, "Authentication error, reconnecting");
                    return (true, None);
                } else {
                    warn!(error = ?data, "Fatal authentication error, shutting down");
                    return (false, None);
                }
            }
            Err(e) => {
                warn!(error = %e, "Shard exited with error, reconnecting");
                return (true, None);
            }
        }
    }
}

#[async_trait]
impl<T: EventForwarder> ShardManager for PublicShardManager<T> {
    #[tracing::instrument(skip(self))]
    async fn connect(self: Arc<Self>) {
        for shard_id in self.options.shard_count.lowest..self.options.shard_count.highest {
            let (ready_tx, ready_rx) = oneshot::channel::<()>();
            let sm = Arc::clone(&self);

            debug!("Fetching resume data");
            let resume_data = match self.session_store.get(shard_id.into()).await {
                Ok(data) => data,
                Err(e) => {
                    error!(error = %e, "Failed to get session data"); // Continue
                    None
                }
            };

            tokio::spawn(async move {
                let mut ready_tx = Some(ready_tx);
                let mut resume_data = resume_data;

                loop {
                    let (reconnect, new_resume_data) = sm
                        .as_ref()
                        .start_shard(shard_id, resume_data.clone(), ready_tx.take())
                        .await;

                    if !reconnect {
                        warn!(%shard_id, "Shard exited with fatal error, not restarting");
                        break;
                    }

                    resume_data = new_resume_data;

                    sleep(Duration::from_millis(500)).await;
                }
            });

            match ready_rx.await {
                Ok(_) => info!(shard_id = %shard_id, "Loaded guilds"),
                Err(e) => error!(shard_id = %shard_id, error = %e, "Error reading ready rx"),
            }
        }

        File::create("/tmp/ready").await.unwrap(); // panic if can't create
        info!("Reported readiness to probe");
    }

    #[tracing::instrument(skip(self))]
    async fn shutdown(self: Arc<Self>) {
        let cluster_size = self.options.shard_count.highest - self.options.shard_count.lowest;
        let (tx, mut rx) = mpsc::channel(cluster_size.into());

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
                    let shard_id = identifier.shard_id;
                    info!(shard_id = %shard_id, "Shard sent None session data");
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

            sessions.insert(identifier.shard_id.into(), session_data);
        }

        if let Err(e) = self.session_store.set_bulk(sessions).await {
            error!(error = %e, "Failed to save session data");
        }

        if let Err(e) = self.event_forwarder.flush().await {
            error!(error = %e, "Failed to flush event forwarder");
        }
    }
}
