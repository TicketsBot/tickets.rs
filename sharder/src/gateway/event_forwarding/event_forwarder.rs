use async_trait::async_trait;
use model::Snowflake;
use crate::gateway::payloads::event::Event;
use crate::{GatewayError, Shard, Config};
use common::event_forwarding;
use std::sync::Arc;

#[async_trait]
pub trait EventForwarder: Sync + Send + 'static {
    async fn forward_event(&self, config: &Config, event: event_forwarding::Event<'_>, guild_id: Option<Snowflake>) -> Result<(), GatewayError>;
}
