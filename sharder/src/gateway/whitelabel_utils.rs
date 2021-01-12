use crate::{Shard, GatewayError};
use std::sync::Arc;
use model::Snowflake;

#[cfg(feature = "whitelabel")]
impl Shard {
    pub async fn store_whitelabel_guild(&self, guild_id: Snowflake) -> Result<(), GatewayError> {
        self.database.whitelabel_guilds.insert(self.user_id, guild_id).await.map_err(GatewayError::DatabaseError)
    }
}

pub fn is_whitelabel() -> bool {
    cfg!(feature = "whitelabel")
}