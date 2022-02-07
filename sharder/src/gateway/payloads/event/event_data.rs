use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;

use chrono::{DateTime, Utc};

use crate::gateway::ShardInfo;
use crate::gateway::util::*;
use model::channel::{Channel, ChannelType, ThreadMember};
use model::guild::{Emoji, Member, Role, UnavailableGuild, VerificationLevel, DefaultMessageNotifications, ExplicitContentFilterLevel, MFALevel, PremiumTier};
use model::user::{PresenceUpdate, User};
use model::{Snowflake, ImageHash};
use cache::model::{CachedGuild, CachedMember};

#[derive(Serialize, Deserialize, Debug)]
pub struct Ready {
    #[serde(rename = "v")]
    pub gateway_version: i32,
    pub user: User,
    pub guilds: Vec<UnavailableGuild>,
    pub session_id: String,
    pub shard: ShardInfo,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ThreadDelete {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub parent_id: Snowflake,
    pub r#type: ChannelType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ThreadListSync {
    pub guild_id: Snowflake,
    pub channel_ids: Option<Vec<Snowflake>>,
    pub threads: Vec<Channel>,
    pub members: Vec<ThreadMember>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ThreadMembersUpdate {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub member_count: u16,
    pub added_members: Option<Vec<ThreadMember>>,
    pub removed_member_ids: Option<Vec<Snowflake>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChannelPinsUpdate {
    pub guild_id: Option<Snowflake>,
    pub channel_id: Snowflake,
    pub last_pin_timestamp: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildUpdate {
    pub id: Snowflake,
    pub name: Option<Box<str>>,
    pub icon: Option<ImageHash>,
    pub afk_channel_id: Option<Snowflake>,
    pub owner_id: Option<Snowflake>,
    pub splash: Option<ImageHash>,
    pub discovery_splash: Option<ImageHash>,
    pub afk_timeout: Option<u16>,
    pub verification_level: Option<VerificationLevel>,
    pub default_message_notifications: Option<DefaultMessageNotifications>,
    pub explicit_content_filter: Option<ExplicitContentFilterLevel>,
    pub features: Option<Vec<Box<str>>>,
    pub mfa_level: Option<MFALevel>,
    pub widget_enabled: Option<bool>,
    pub widget_channel_id: Option<Snowflake>,
    pub system_channel_id: Option<Snowflake>,
    pub vanity_url_code: Option<Box<str>>,
    pub description: Option<Box<str>>,
    pub banner: Option<ImageHash>,
    pub premium_tier: Option<PremiumTier>,
    pub premium_subscription_count: Option<u32>,
    pub unavailable: Option<bool>,
}

#[cfg(feature = "memory-cache")]
impl GuildUpdate {
    pub fn apply_changes(self, target: &mut CachedGuild) {
        replace_if_some(&mut target.name, self.name);
        replace_option_if_some(&mut target.icon, self.icon);
        replace_option_if_some(&mut target.afk_channel_id, self.afk_channel_id);
        replace_if_some(&mut target.owner_id, self.owner_id);
        replace_option_if_some(&mut target.splash, self.splash);
        replace_option_if_some(&mut target.discovery_splash, self.discovery_splash);
        replace_if_some(&mut target.afk_timeout, self.afk_timeout);
        replace_if_some(&mut target.verification_level, self.verification_level);
        replace_if_some(&mut target.default_message_notifications, self.default_message_notifications);
        replace_if_some(&mut target.explicit_content_filter, self.explicit_content_filter);
        replace_if_some(&mut target.features, self.features);
        replace_if_some(&mut target.mfa_level, self.mfa_level);
        replace_if_some(&mut target.widget_enabled, self.widget_enabled);
        replace_option_if_some(&mut target.widget_channel_id, self.widget_channel_id);
        replace_option_if_some(&mut target.system_channel_id, self.system_channel_id);
        replace_option_if_some(&mut target.vanity_url_code, self.vanity_url_code);
        replace_option_if_some(&mut target.description, self.description);
        replace_option_if_some(&mut target.banner, self.banner);
        replace_if_some(&mut target.premium_tier, self.premium_tier);
        replace_if_some(&mut target.premium_subscription_count, self.premium_subscription_count);
        replace_option_if_some(&mut target.unavailable, self.unavailable);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildBanAdd {
    pub guild_id: Snowflake,
    pub user: User,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildBanRemove {
    pub guild_id: Snowflake,
    pub user: User,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildEmojisUpdate {
    pub guild_id: Snowflake,
    pub emojis: Vec<Emoji>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildIntegrationsUpdate {
    pub guild_id: Snowflake,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildJoinRequestDelete {
    pub user_id: Snowflake,
    pub guild_id: Snowflake,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildMemberAdd {
    pub guild_id: Snowflake,
    #[serde(flatten)]
    pub member: Member,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildMemberRemove {
    pub guild_id: Snowflake,
    pub user: User,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildMemberUpdate {
    pub guild_id: Snowflake,
    pub roles: Vec<Snowflake>,
    pub user: User,
    pub nick: Option<Box<str>>,
    pub joined_at: Option<DateTime<Utc>>,
    pub premium_since: Option<DateTime<Utc>>,
    pub deaf: Option<bool>,
    pub mute: Option<bool>,
    pub pending: Option<bool>,
}

impl GuildMemberUpdate {
    pub fn apply_changes(self, target: &mut CachedMember) {
        target.roles = self.roles;
        target.nick = self.nick;
        replace_if_some(&mut target.joined_at, self.joined_at);
        target.premium_since = self.premium_since;

        replace_if_some(&mut target.deaf, self.deaf);
        replace_if_some(&mut target.mute, self.mute);
        replace_if_some(&mut target.pending, self.pending);
    }
}

impl Into<Member> for GuildMemberUpdate {
    fn into(self) -> Member {
        Member {
            user: Some(self.user),
            nick: self.nick,
            roles: self.roles,
            joined_at: self.joined_at.unwrap_or_else(|| Utc::now()), // TODO: Improve
            premium_since: self.premium_since,
            deaf: self.deaf.unwrap_or(false),
            mute: self.mute.unwrap_or(false),
            pending: self.pending.unwrap_or(false),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildMembersChunk {
    pub guild_id: Snowflake,
    pub members: Vec<Member>,
    pub chunk_index: u32,
    pub chunk_count: u32,
    pub not_found: Option<Vec<Snowflake>>,
    pub presences: Option<Vec<PresenceUpdate>>,
    pub nonce: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildRoleCreate {
    pub guild_id: Snowflake,
    pub role: Role,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildRoleUpdate {
    pub guild_id: Snowflake,
    pub role: Role,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GuildRoleDelete {
    pub guild_id: Snowflake,
    pub role_id: Snowflake,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InviteCreate {
    pub channel_id: Snowflake,
    pub code: String,
    pub created_at: DateTime<Utc>,
    pub guild_id: Option<Snowflake>,
    pub inviter: Option<User>,
    pub max_age: usize,
    pub max_uses: u32,
    pub target_user: Option<User>,
    pub target_user_type: Option<TargetUserType>,
    pub temporary: bool,
    pub uses: u8, // we can use u8 because it will always initially be 0
}

#[derive(Serialize, Deserialize_repr, Debug)]
#[repr(u8)]
pub enum TargetUserType {
    Stream = 1,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InviteDelete {
    pub channel_id: Snowflake,
    pub guild_id: Option<Snowflake>,
    pub code: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageDelete {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    pub guild_id: Option<Snowflake>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageDeleteBulk {
    pub ids: Vec<Snowflake>,
    pub channel_id: Snowflake,
    pub guild_id: Option<Snowflake>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageReactionAdd {
    pub user_id: Snowflake,
    pub channel_id: Snowflake,
    pub message_id: Snowflake,
    pub guild_id: Option<Snowflake>,
    pub member: Option<Member>,
    pub emoji: Emoji,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageReactionRemove {
    pub user_id: Snowflake,
    pub channel_id: Snowflake,
    pub message_id: Snowflake,
    pub guild_id: Option<Snowflake>,
    pub emoji: Emoji,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageReactionRemoveAll {
    pub channel_id: Snowflake,
    pub message_id: Snowflake,
    pub guild_id: Option<Snowflake>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageReactionRemoveEmoji {
    pub channel_id: Snowflake,
    pub guild_id: Option<Snowflake>,
    pub message_id: Snowflake,
    pub emoji: Emoji,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TypingStart {
    pub channel_id: Snowflake,
    pub guild_id: Option<Snowflake>,
    pub user_id: Snowflake,
    pub timestamp: u64,
    pub member: Option<Member>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoiceServerUpdate {
    pub token: String,
    pub guild_id: Snowflake,
    pub endpoint: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WebhooksUpdate {
    pub guild_id: Snowflake,
    pub channel_id: Snowflake,
}
