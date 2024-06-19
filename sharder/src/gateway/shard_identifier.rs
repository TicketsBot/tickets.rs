use model::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ShardIdentifier {
    pub bot_id: Snowflake,
    pub shard_id: u16,
}

impl ShardIdentifier {
    pub fn new(bot_id: Snowflake, shard_id: u16) -> Self {
        Self { bot_id, shard_id }
    }
}
