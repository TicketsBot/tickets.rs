use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

pub const EVENT_KEY: &str = "tickets:events";

#[derive(Deserialize, Serialize, Debug)]
pub struct Event {
    pub bot_token: String,
    pub bot_id: u64,
    pub is_whitelabel: bool,
    pub shard_id: u16,
    pub event: Box<RawValue>,
}
