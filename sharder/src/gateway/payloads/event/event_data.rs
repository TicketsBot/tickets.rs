use serde::{Deserialize, Serialize};
use serde_repr::Deserialize_repr;

use chrono::{DateTime, Utc};

use crate::gateway::ShardInfo;
use model::channel::{Channel, ChannelType, ThreadMember};
use model::guild::{Emoji, Member, Role, UnavailableGuild};
use model::user::{PresenceUpdate, User};
use model::Snowflake;

#[derive(Serialize, Deserialize, Debug)]
pub struct Ready {
    #[serde(rename = "v")]
    pub gateway_version: i32,
    pub user: User,
    pub guilds: Vec<UnavailableGuild>,
    pub session_id: String,
    pub resume_gateway_url: String,
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
pub struct GuildJoinRequestUpdate {
    pub status: String,
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
    pub nick: Option<String>,
    pub joined_at: DateTime<Utc>,
    pub premium_since: Option<DateTime<Utc>>,
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
pub struct WebhookUpdate {
    pub guild_id: Snowflake,
    pub channel_id: Snowflake,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VoiceChannelStatusUpdate {
    id: Snowflake,
    guild_id: Snowflake,
    status: Option<String>,
    old: Option<String>,
}
