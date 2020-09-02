use serde::Deserialize;
use serde_repr::Deserialize_repr;

use crate::model::Snowflake;
use crate::model::user::User;
use crate::model::guild::Member;
use chrono::{DateTime, Utc};
use crate::model::channel::{ChannelType, Reaction};
use super::embed::Embed;

#[derive(Deserialize, Debug)]
pub struct Message {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    pub guild_id: Option<Snowflake>,
    pub author: Option<User>,
    pub member: Option<Member>,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub edited_timestamp: Option<DateTime<Utc>>,
    pub tts: bool,
    pub mention_everyone: bool,
    pub mentions: Vec<MentionedUser>,
    pub mention_roles: Vec<Snowflake>,
    pub mention_channels: Option<Vec<ChannelMention>>,
    pub attachments: Vec<Attachment>,
    pub embed: Vec<Embed>,
    pub reactions: Option<Vec<Reaction>>,
    pub nonce: Option<String>,
    pub pinned: bool,
    pub webhook_id: Option<Snowflake>,
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub activity: Option<MessageActivity>,
    pub application: Option<MessageApplication>,
    pub message_reference: Option<MessageReference>,
    pub flags: u32,
}

#[derive(Deserialize, Debug)]
pub struct MentionedUser {
    #[serde(flatten)]
    pub user: User,
    pub member: Member,
}

#[derive(Deserialize, Debug)]
pub struct ChannelMention {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct Attachment {
    pub id: Snowflake,
    pub filename: String,
    pub size: usize,
    pub url: String,
    pub proxy_url: String,
    pub height: Option<usize>,
    pub width: Option<usize>,
}

#[derive(Deserialize_repr, Debug)]
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

#[derive(Deserialize, Debug)]
pub struct MessageActivity {
    #[serde(rename = "type")]
    pub activity_type: MessageActivityType,
    pub party_id: Option<String>,
}

#[derive(Deserialize_repr, Debug)]
#[repr(u8)]
pub enum MessageActivityType {
    Join = 1,
    Spectate = 2,
    Listen = 3,
    JoinRequest = 5,
}

#[derive(Deserialize, Debug)]
pub struct MessageApplication {
    pub id: Snowflake,
    pub cover_image: Option<String>,
    pub description: String,
    pub icon: Option<String>,
    pub name: String,
}

#[derive(Deserialize, Debug)]
pub struct MessageReference {
    pub message_id: Option<Snowflake>,
    pub channel_id: Snowflake,
    pub guild_id: Option<Snowflake>,
}