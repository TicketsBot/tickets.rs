use serde::{Deserialize, Serialize};

use super::Member;
use crate::Snowflake;

#[derive(Serialize, Deserialize, Debug)]
pub struct VoiceState {
    #[serde(skip_serializing)]
    pub guild_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    #[serde(skip_serializing)]
    pub user_id: Snowflake,
    #[serde(skip_serializing)]
    pub member: Option<Member>,
    pub session_id: String,
    pub deaf: bool,
    pub mute: bool,
    pub self_deaf: bool,
    pub self_mute: bool,
    pub self_stream: Option<bool>,
    pub self_video: bool,
    pub suppress: bool,
}

impl PartialEq for VoiceState {
    fn eq(&self, other: &Self) -> bool {
        self.guild_id == other.guild_id && self.channel_id == other.channel_id
    }
}
