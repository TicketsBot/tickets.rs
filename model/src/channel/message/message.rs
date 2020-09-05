use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};

use crate::Snowflake;
use crate::user::User;
use crate::guild::Member;
use chrono::{DateTime, Utc};
use crate::channel::{ChannelType, Reaction};
use super::embed::Embed;

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    pub guild_id: Option<Snowflake>,
    pub author: Option<User>,
    pub member: Option<Member>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Box<str>>,
    pub timestamp: Option<DateTime<Utc>>,
    pub edited_timestamp: Option<DateTime<Utc>>,
    #[serde(default)]
    pub tts: bool,
    #[serde(default)]
    pub mention_everyone: bool,
    #[serde(default)]
    pub mentions: Vec<MentionedUser>,
    #[serde(default)]
    pub mention_roles: Vec<Snowflake>,
    #[serde(default)]
    pub mention_channels: Vec<ChannelMention>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embed: Option<Vec<Embed>>,
    #[serde(default)]
    pub reactions: Vec<Reaction>,
    pub nonce: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    pub webhook_id: Option<Snowflake>,
    #[serde(rename = "type", default)]
    pub message_type: MessageType,
    pub activity: Option<MessageActivity>,
    pub application: Option<MessageApplication>,
    pub message_reference: Option<MessageReference>,
    #[serde(default)]
    pub flags: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MentionedUser {
    #[serde(flatten)]
    pub user: User,
    pub member: Option<Member>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChannelMention {
    pub id: Snowflake,
    pub guild_id: Option<Snowflake>,
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Attachment {
    pub id: Snowflake,
    pub filename: String,
    pub size: usize,
    pub url: String,
    pub proxy_url: String,
    pub height: Option<usize>,
    pub width: Option<usize>,
}

#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum MessageType {
    Default = 0,
    RecipientAdd = 1,
    RecipientRemove = 2,
    Call = 3,
    ChannelNameChange = 4,
    ChannelIconChange = 5,
    ChannelPinnedMessage = 6,
    GuildMemberJoin = 7,
    UserPremiumGuildSubscription = 8,
    UserPremiumGuildSubscriptionOne = 9,
    UserPremiumGuildSubscriptionTwo = 10,
    UserPremiumGuildSubscriptionThree = 11,
    ChannelFollowAdd = 12,
    GuildDiscoveryDisqualified = 14,
    GuildDiscoveryRequalified = 15,
}

impl Default for MessageType {
    fn default() -> Self {
        MessageType::Default
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageActivity {
    #[serde(rename = "type")]
    pub activity_type: MessageActivityType,
    pub party_id: Option<String>,
}

#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum MessageActivityType {
    Join = 1,
    Spectate = 2,
    Listen = 3,
    JoinRequest = 5,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageApplication {
    pub id: Snowflake,
    pub cover_image: Option<String>,
    pub description: String,
    pub icon: Option<String>,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageReference {
    pub message_id: Option<Snowflake>,
    pub channel_id: Snowflake,
    pub guild_id: Option<Snowflake>,
}