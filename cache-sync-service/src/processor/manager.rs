use std::sync::Arc;

use cache::PostgresCache;
use event_stream::Consumer;
use tracing::debug;

use crate::{Config, Result};

use super::worker::Worker;

pub struct Manager {
    config: Config,
    cache: Arc<PostgresCache>,
}

impl Manager {
    pub fn new(config: Config, cache: PostgresCache) -> Self {
        let cache = Arc::new(cache);

        Self { config, cache }
    }

    pub fn start(&self) -> Result<()> {
        let topic = self.config.topic.clone();

        debug!(%topic, "Connecting consumer");
        let consumer = Arc::new(Consumer::new(
            self.config.brokers.clone(),
            topic.clone(),
            self.config.group_id.clone(),
        )?);

        debug!("Consumer connected!");

        for i in 0..self.config.workers {
            let worker = Worker::new(
                i,
                self.config.batch_size,
                Arc::clone(&consumer),
                Arc::clone(&self.cache),
            );

            tokio::spawn(async move {
                worker.run().await;
            });
        }

        Ok(())
    }
}
