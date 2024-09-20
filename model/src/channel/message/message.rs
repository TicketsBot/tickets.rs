use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::embed::Embed;
use crate::channel::{Channel, ChannelType, Reaction};
use crate::guild::Member;
use crate::interaction::{Component, InteractionType};
use crate::user::User;
use crate::util::empty_vec;
use crate::Snowflake;
use chrono::{DateTime, Utc};

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
    pub nonce: Option<serde_json::Value>,
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
    pub referenced_message: Box<Option<Message>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default = "empty_vec")]
    pub components: Vec<Component>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction: Option<MessageInteraction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread: Option<Channel>,
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
    pub description: Option<String>,
    pub content_type: Option<String>,
    pub size: usize,
    pub url: String,
    pub proxy_url: String,
    pub height: Option<usize>,
    pub width: Option<usize>,
    #[serde(default)]
    pub ephemeral: bool,
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
    GuildDiscoveryGracePeriodInitialWarning = 16,
    GuildDiscoveryGracePeriodFinalWarning = 17,
    ThreadCreated = 18,
    Reply = 19,
    ChatInputCommand = 20,
    ThreadStarterMessage = 21,
    GuildInviteReminder = 22,
    ContextMenuCommand = 23,
    AutoModerationAction = 24,
    RoleSubscriptionPurchase = 25,
    InteractionPremiumUpsell = 26,
    StageStart = 27,
    StageEnd = 28,
    StageSpeaker = 29,
    StageTopic = 31,
    GuildApplicationPremiumSubscription = 32,
    GuildIncidentAlertModeEnabled = 36,
    GuildIncidentAlertModeDisabled = 37,
    GuildIncidentReportRaid = 38,
    GuildIncidentReportFalseAlarm = 39,
    PurchaseNotification = 44,
    PollResult = 46,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageInteraction {
    pub id: Snowflake,
    pub r#type: InteractionType,
    pub name: Box<str>,
    pub user: User,
}
