#[cfg(feature = "whitelabel")]
use crate::gateway::event_forwarding::EventForwarder;
#[cfg(feature = "whitelabel")]
use crate::{GatewayError, Result, Shard};
#[cfg(feature = "whitelabel")]
use model::Snowflake;

#[cfg(feature = "whitelabel")]
impl<T: EventForwarder> Shard<T> {
    pub async fn store_whitelabel_guild(&self, guild_id: Snowflake) -> Result<()> {
        self.database
            .whitelabel_guilds
            .insert(self.user_id, guild_id)
            .await
            .map_err(GatewayError::DatabaseError)
    }
}

pub fn is_whitelabel() -> bool {
    cfg!(feature = "whitelabel")
}
