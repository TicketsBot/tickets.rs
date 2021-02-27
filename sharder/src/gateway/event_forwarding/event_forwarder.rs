use async_trait::async_trait;
use model::Snowflake;
use crate::{GatewayError, Config};
use common::event_forwarding;

#[async_trait]
pub trait EventForwarder: Sync + Send + 'static {
    async fn forward_event(&self, config: &Config, event: event_forwarding::Event<'_>, guild_id: Option<Snowflake>) -> Result<(), GatewayError>;
}
