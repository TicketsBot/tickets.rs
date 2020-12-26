use serde::{Serialize, Deserialize};
use serde_repr::{Serialize_repr, Deserialize_repr};

use crate::{Snowflake, ImageHash, PermissionBitSet};
use super::{Role, Emoji, VoiceState, Member};
use chrono::{DateTime, Utc};
use crate::channel::Channel;
use crate::user::PresenceUpdate;

#[derive(Serialize, Deserialize, Debug)]
pub struct Guild {
    #[serde(skip_serializing)]
    pub id: Snowflake,
    pub name: String,
    pub icon: Option<ImageHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splash: Option<ImageHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_splash: Option<ImageHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<bool>,
    #[serde(serialize_with = "Snowflake::serialize_to_int")]
    pub owner_id: Snowflake,
    pub permissions: Option<PermissionBitSet>,
    pub region: String,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "Snowflake::serialize_option_to_int")]
    pub afk_channel_id: Option<Snowflake>,
    pub afk_timeout: u16,
    pub verification_level: VerificationLevel,
    pub default_message_notifications: DefaultMessageNotifications,
    pub explicit_content_filter: ExplicitContentFilterLevel,
    #[serde(skip_serializing, default)]
    pub roles: Vec<Role>,
    //#[serde(skip_serializing, default)]
    #[serde(skip)]
    pub emojis: Vec<Emoji>,
    pub features: Vec<Features>,
    pub mfa_level: MFALevel,
    pub application_id: Option<Snowflake>,
    pub widget_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "Snowflake::serialize_option_to_int")]
    pub widget_channel_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "Snowflake::serialize_option_to_int")]
    pub system_channel_id: Option<Snowflake>,
    pub system_channels_flags: Option<u8>,
    pub rules_channel_id: Option<Snowflake>,
    pub joined_at: Option<DateTime<Utc>>,
    pub large: Option<bool>,
    pub unavailable: Option<bool>,
    pub member_count: Option<u32>,
    //#[serde(skip_serializing)]
    #[serde(skip)]
    pub voice_states: Option<Vec<VoiceState>>,
    #[serde(skip_serializing)]
    pub members: Option<Vec<Member>>,
    #[serde(skip_serializing)]
    pub channels: Option<Vec<Channel>>,
    #[serde(skip_serializing)]
    pub presences: Option<Vec<PresenceUpdate>>,
    pub max_presences: Option<u32>,
    pub max_members: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vanity_url_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<ImageHash>,
    pub premium_tier: PremiumTier,
    pub premium_subscription_count: Option<u16>,
    pub preferred_locale: String,
    #[serde(skip_serializing_if = "Option::is_none", serialize_with = "Snowflake::serialize_option_to_int")]
    pub public_updates_channel_id: Option<Snowflake>,
    pub max_video_channel_users: Option<u8>,
    pub approximate_member_count: Option<u32>,
    pub approximate_presence_count: Option<u32>,
}

impl PartialEq for Guild {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}


#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum VerificationLevel {
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    VeryHigh = 4,
}

#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum DefaultMessageNotifications {
    AllMessage = 0,
    OnlyMentions = 1,
}

#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum ExplicitContentFilterLevel {
    Disabled = 0,
    MembersWithoutRoles = 1,
    AllMembers = 2,
}

#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum MFALevel {
    None = 0,
    Elevated = 1,
}

#[derive(Serialize_repr, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum PremiumTier {
    None = 0,
    TierOne = 1,
    TierTwo = 2,
    TierThree = 3,
}

#[derive(Serialize, Deserialize, Debug)]
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
}