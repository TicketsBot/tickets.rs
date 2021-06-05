use crate::channel::message::embed::Embed;
use crate::channel::message::AllowedMentions;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct InteractionApplicationCommandCallbackData {
    pub tts: Option<bool>,
    pub content: Box<str>,
    pub embeds: Option<Vec<Embed>>,
    pub allowed_mentions: Option<Vec<AllowedMentions>>,
    pub flags: u32,
}
