use async_trait::async_trait;
use tracing::{error, info, warn};

use super::Options;
use super::ShardManager;

use crate::gateway::{Identify, Shard, ShardInfo};
use crate::SessionData;
use crate::{RedisSessionStore, SessionStore};

use std::collections::HashMap;
use std::sync::Arc;

use cache::PostgresCache;

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
    cache: Arc<PostgresCache>,
    redis: Arc<Pool>,
    event_forwarder: Arc<T>,
    shutdown_tx: broadcast::Sender<mpsc::Sender<(u16, Option<SessionData>)>>,
}

impl<T: EventForwarder> PublicShardManager<T> {
    pub async fn new(
        config: Config,
        options: Options,
        session_store: RedisSessionStore,
        cache: Arc<PostgresCache>,
        redis: Arc<Pool>,
        event_forwarder: Arc<T>,
    ) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            config: Arc::new(config),
            options,
            session_store,
            cache,
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
            Arc::clone(&self.cache),
            Arc::clone(&self.redis),
            self.options.user_id,
            Arc::clone(&self.event_forwarder),
            ready_tx,
            self.shutdown_tx.subscribe(),
        )
    }
}

#[async_trait]
impl<T: EventForwarder> ShardManager for PublicShardManager<T> {
    async fn connect(self: Arc<Self>) {
        for shard_id in self.options.shard_count.lowest..self.options.shard_count.highest {
            let (ready_tx, ready_rx) = oneshot::channel::<()>();
            let sm = Arc::clone(&self);

            tokio::spawn(async move {
                let mut ready_tx = Some(ready_tx);

                let mut resume_data = sm
                    .session_store
                    .get(shard_id.into())
                    .await
                    .expect("Failed to fetch session data"); // TODO: Log, not panic

                loop {
                    let shard = sm.build_shard(shard_id, ready_tx.take());
                    shard.log("Starting...");

                    // TODO: Skip ready_rx await on error
                    match shard.connect(resume_data.clone()).await {
                        Ok(session_data) => {
                            resume_data = session_data;
                            info!(shard_id = %shard_id, "Shard exited normally");
                        }
                        Err(GatewayError::AuthenticationError { data, .. }) => {
                            if data.should_reconnect() {
                                warn!(shard_id = %shard_id, error = ?data, "Authentication error, reconnecting");
                            } else {
                                warn!(shard_id = %shard_id, error = ?data, "Fatal authentication error, shutting down");
                                break;
                            }
                        }
                        Err(e) => {
                            warn!(shard_id = %shard_id, error = %e, "Shard exited with error, reconnecting")
                        }
                    }

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

    async fn shutdown(self: Arc<Self>) {
        let cluster_size = self.options.shard_count.highest - self.options.shard_count.lowest;
        let (tx, mut rx) = mpsc::channel(cluster_size.into());

        let receivers = self
            .shutdown_tx
            .send(tx)
            .expect("Failed to send shutdown signal to shards");

        let mut sessions = HashMap::new();
        for _ in 0..receivers {
            let (shard_id, session_data) = match timeout(Duration::from_secs(30), rx.recv()).await {
                Ok(Some((shard_id, Some(session_data)))) => (shard_id, session_data),
                Ok(Some((shard_id, None))) => {
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

            sessions.insert(shard_id.into(), session_data);
        }

        if let Err(e) = self.session_store.set_bulk(sessions).await {
            error!(error = %e, "Failed to save session data");
        }
    }
}
