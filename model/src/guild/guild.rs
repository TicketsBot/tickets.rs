use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use super::{Emoji, Member, Role, VoiceState};
use crate::channel::Channel;
use crate::user::PresenceUpdate;
use crate::{ImageHash, Snowflake};
use chrono::{DateTime, Utc};
use crate::stage::StageInstance;
use crate::sticker::Sticker;

#[derive(Serialize, Deserialize, Debug)]
pub struct Guild {
    pub id: Snowflake,
    pub name: Box<str>,
    pub icon: Option<ImageHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splash: Option<ImageHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_splash: Option<ImageHash>,
    #[serde(serialize_with = "Snowflake::serialize_to_int")]
    pub owner_id: Snowflake,
    #[serde(serialize_with = "Snowflake::serialize_option_to_int")]
    pub afk_channel_id: Option<Snowflake>,
    pub afk_timeout: u16,
    #[serde(default)]
    pub widget_enabled: bool,
    #[serde(serialize_with = "Snowflake::serialize_option_to_int")]
    pub widget_channel_id: Option<Snowflake>,
    pub verification_level: VerificationLevel,
    pub default_message_notifications: DefaultMessageNotifications,
    pub explicit_content_filter: ExplicitContentFilterLevel,
    pub roles: Vec<Role>,
    pub emojis: Vec<Emoji>,
    #[serde(default)]
    pub features: Vec<Box<str>>,
    pub mfa_level: MFALevel,
    #[serde(serialize_with = "Snowflake::serialize_option_to_int")]
    pub application_id: Option<Snowflake>,
    #[serde(serialize_with = "Snowflake::serialize_option_to_int")]
    pub system_channel_id: Option<Snowflake>,
    #[serde(default)]
    pub system_channels_flags: u32,
    #[serde(serialize_with = "Snowflake::serialize_option_to_int")]
    pub rules_channel_id: Option<Snowflake>,
    pub joined_at: DateTime<Utc>,
    pub large: bool,
    pub unavailable: Option<bool>,
    pub member_count: u32,
    pub voice_states: Vec<VoiceState>,
    pub members: Vec<Member>,
    pub channels: Vec<Channel>,
    pub threads: Vec<Channel>,
    pub presences: Vec<PresenceUpdate>,
    pub max_presences: Option<u32>,
    pub max_members: u32,
    pub vanity_url_code: Option<Box<str>>,
    pub description: Option<Box<str>>,
    pub banner: Option<ImageHash>,
    pub premium_tier: PremiumTier,
    #[serde(default)]
    pub premium_subscription_count: u32,
    pub preferred_locale: Box<str>,
    #[serde(serialize_with = "Snowflake::serialize_option_to_int")]
    pub public_updates_channel_id: Option<Snowflake>,
    pub max_video_channel_users: u16,
    #[serde(default)]
    pub approximate_member_count: u32,
    #[serde(default)]
    pub approximate_presence_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub welcome_screen: Option<WelcomeScreen>,
    #[serde(default = "NsfwLevel::default")]
    pub nsfw_level: NsfwLevel,
    pub stage_instances: Vec<StageInstance>,
    pub stickers: Vec<Sticker>,
    pub premium_progress_bar_enabled: bool,
}

impl PartialEq for Guild {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone)]
#[repr(u8)]
pub enum VerificationLevel {
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    VeryHigh = 4,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone)]
#[repr(u8)]
pub enum DefaultMessageNotifications {
    AllMessage = 0,
    OnlyMentions = 1,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone)]
#[repr(u8)]
pub enum ExplicitContentFilterLevel {
    Disabled = 0,
    MembersWithoutRoles = 1,
    AllMembers = 2,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone)]
#[repr(u8)]
pub enum MFALevel {
    None = 0,
    Elevated = 1,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone)]
#[repr(u8)]
pub enum PremiumTier {
    None = 0,
    TierOne = 1,
    TierTwo = 2,
    TierThree = 3,
}

/*#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Features {
    InviteSplash,
    VipRegions,
    VanityUrl,
    Verified,
    Partnered,
    Public,
    Commerce,
    News,
    Discoverable,
    Featurable,
    AnimatedIcon,
    Banner,
    PublicDisabled,
    WelcomeScreenEnabled,
    Community,
    EnabledDiscoverableBefore,
    PreviewEnabled,
    MemberVerificationGateEnabled,
    DiscoverableDisabled,
    PrivateThreads,
    ThreeDayThreadArchive,
    SevenDayThreadArchive,
    ThreadsEnabled,
}*/

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WelcomeScreen {
    pub description: Option<Box<str>>,
    pub welcome_channels: Vec<WelcomeScreenChannel>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WelcomeScreenChannel {
    pub channel_id: Snowflake,
    pub description: Box<str>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "Snowflake::serialize_option_to_int"
    )]
    pub emoji_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<Box<str>>,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, Copy, Clone)]
#[repr(u8)]
pub enum NsfwLevel {
    Default = 0,
    Explicit = 1,
    Safe = 2,
    AgeRestricted = 3,
}

impl Default for NsfwLevel {
    fn default() -> Self {
        NsfwLevel::Default
    }
}
