use serde::Deserialize;
use serde_repr::Deserialize_repr;

use crate::model::{Snowflake, ImageHash, PermissionBitSet};
use super::{Role, Emoji, VoiceState, Member};
use chrono::{DateTime, Utc};
use crate::model::channel::Channel;
use crate::model::user::{PresenceUpdate, PremiumType};

#[derive(Deserialize, Debug)]
pub struct Guild {
    pub id: Snowflake,
    pub name: String,
    pub icon: Option<ImageHash>,
    pub splash: Option<ImageHash>,
    pub discovery_splash: Option<ImageHash>,
    pub owner: Option<bool>,
    pub owner_id: Snowflake,
    pub permissions: Option<u32>,
    pub permissions_new: Option<PermissionBitSet>,
    pub region: String,
    pub afk_channel_id: Option<Snowflake>,
    pub afk_timeout: u16,
    pub embed_enabled: Option<bool>,
    pub embed_channel_id: Option<Snowflake>,
    pub verification_level: VerificationLevel,
    pub default_message_notifications: DefaultMessageNotifications,
    pub explicit_content_filter: ExplicitContentFilterLevel,
    pub roles: Vec<Role>,
    pub emojis: Vec<Emoji>,
    pub features: Vec<Features>,
    pub mfa_level: MFALevel,
    pub application_id: Option<Snowflake>,
    pub widget_enabled: Option<bool>,
    pub widget_channel_id: Option<Snowflake>,
    pub system_channel_id: Option<Snowflake>,
    pub system_channels_flags: u8,
    pub rules_channel_id: Option<Snowflake>,
    pub joined_at: Option<DateTime<Utc>>,
    pub large: Option<bool>,
    pub unavailable: Option<bool>,
    pub member_count: Option<u32>,
    pub voice_states: Option<Vec<VoiceState>>,
    pub members: Option<Vec<Member>>,
    pub channels: Option<Vec<Channel>>,
    pub presences: Option<Vec<PresenceUpdate>>,
    pub max_presences: Option<u32>,
    pub max_members: Option<u32>,
    pub vanity_url_code: Option<String>,
    pub description: Option<String>,
    pub banner: Option<ImageHash>,
    pub premium_tier: PremiumType,
    pub premium_subscription_count: Option<u16>,
    pub preferred_locale: String,
    pub public_updates_channel_id: Option<Snowflake>,
    pub max_video_channel_users: Option<u8>,
    pub approximate_member_count: Option<u32>,
    pub approximate_presence_count: Option<u32>,
}

#[derive(Deserialize_repr, Debug)]
#[repr(u8)]
pub enum VerificationLevel {
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    VeryHigh = 4,
}

#[derive(Deserialize_repr, Debug)]
#[repr(u8)]
pub enum DefaultMessageNotifications {
    AllMessage = 0,
    OnlyMentions = 1,
}

#[derive(Deserialize_repr, Debug)]
#[repr(u8)]
pub enum ExplicitContentFilterLevel {
    Disabled = 0,
    MembersWithoutRoles = 1,
    AllMembers = 2,
}

#[derive(Deserialize_repr, Debug)]
#[repr(u8)]
pub enum MFALevel {
    None = 0,
    Elevated = 1,
}

#[derive(Deserialize_repr, Debug)]
#[repr(u8)]
pub enum PremiumTier {
    None = 0,
    TierOne = 1,
    TierTwo = 2,
    TierThree = 3,
}

#[derive(Deserialize, Debug)]
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
}