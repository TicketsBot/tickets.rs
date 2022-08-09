use crate::{Config, Result};
use async_trait::async_trait;
use common::event_forwarding;
use model::Snowflake;

#[async_trait]
pub trait EventForwarder: Sync + Send + 'static {
    async fn forward_event(
        &self,
        config: &Config,
        event: event_forwarding::Event<'_>,
        guild_id: Option<Snowflake>,
    ) -> Result<()>;
}
