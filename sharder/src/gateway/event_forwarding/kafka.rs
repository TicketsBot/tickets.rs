use std::time::Duration;

use async_trait::async_trait;
use common::event_forwarding;
use event_stream::Publisher;
use model::Snowflake;

use crate::{Config, Result};

use super::EventForwarder;

pub struct KafkaEventForwarder {
    publisher: Publisher,
}

impl KafkaEventForwarder {
    pub fn new(config: &Config) -> Result<Self> {
        let publisher = Publisher::new(config.kafka_brokers.clone(), config.kafka_topic.clone())?;

        Ok(Self { publisher })
    }
}

#[async_trait]
impl EventForwarder for KafkaEventForwarder {
    #[tracing::instrument(skip(self, _config, event))]
    async fn forward_event(
        &self,
        _config: &Config,
        event: event_forwarding::Event,
        guild_id: Option<Snowflake>,
    ) -> Result<()> {
        self.publisher
            .send(&event, guild_id.map(|s| s.0).unwrap_or(0))?;
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        self.publisher.flush(Duration::from_secs(5));
        Ok(())
    }
}
