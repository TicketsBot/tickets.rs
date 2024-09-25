mod http;
use async_trait::async_trait;
use common::event_forwarding;
pub use http::HttpEventForwarder;

mod kafka;
pub use kafka::KafkaEventForwarder;

mod util;
use model::Snowflake;
pub use util::{get_guild_id, is_whitelisted};

use crate::{Config, Result};

#[async_trait]
pub trait EventForwarder: Sync + Send + 'static {
    async fn forward_event(
        &self,
        config: &Config,
        event: event_forwarding::Event,
        guild_id: Option<Snowflake>,
    ) -> Result<()>;

    async fn flush(&self) -> Result<()>;
}
