use serde::Serialize;
use serde_json::value::RawValue;

pub const EVENT_KEY: &str = "tickets:events";

#[derive(Serialize, Debug)]
pub struct Event<'a> {
    pub bot_token: &'a str,
    pub bot_id: u64,
    pub is_whitelabel: bool,
    pub shard_id: u16,
    pub event: Box<RawValue>,
}
