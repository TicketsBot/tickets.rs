use serde::Serialize;
use serde_json::value::RawValue;
use model::interaction::InteractionType;

pub const COMMAND_KEY: &str = "tickets:commands";

#[derive(Serialize, Debug)]
pub struct ForwardedInteraction<'a> {
    pub bot_token: &'a str,
    pub bot_id: u64,
    pub is_whitelabel: bool,
    pub interaction_type: InteractionType,
    pub data: Box<RawValue>,
}
