use super::CacheError;

use async_trait::async_trait;
use model::Snowflake;
use model::user::User;
use model::channel::Channel;
use model::guild::{Role, Guild, Member, Emoji, VoiceState};

#[async_trait]
pub trait Cache {
    async fn store_guild(&self, guild: &Guild) -> Result<(), CacheError>;
    async fn store_guilds(&self, guilds: Vec<&Guild>) -> Result<(), CacheError>;
    async fn get_guild(&self, id: Snowflake) -> Result<Option<Guild>, CacheError>;
    async fn delete_guild(&self, id: Snowflake) -> Result<(), CacheError>;

    async fn store_channel(&self, channel: &Channel) -> Result<(), CacheError>;
    async fn store_channels(&self, channels: Vec<&Channel>) -> Result<(), CacheError>;
    async fn get_channel(&self, id: Snowflake) -> Result<Option<Channel>, CacheError>;
    async fn delete_channel(&self, id: Snowflake) -> Result<(), CacheError>;

    async fn store_user(&self, user: &User) -> Result<(), CacheError>;
    async fn store_users(&self, users: Vec<&User>) -> Result<(), CacheError>;
    async fn get_user(&self, id: Snowflake) -> Result<Option<User>, CacheError>;
    async fn delete_user(&self, id: Snowflake) -> Result<(), CacheError>;

    async fn store_member(&self, member: &Member, guild_id: Snowflake) -> Result<(), CacheError>;
    async fn store_members(&self, members: Vec<&Member>, guild_id: Snowflake) -> Result<(), CacheError>;
    async fn get_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<Member>, CacheError>;
    async fn delete_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<(), CacheError>;

    async fn store_role(&self, role: &Role, guild_id: Snowflake) -> Result<(), CacheError>;
    async fn store_roles(&self, roles: Vec<&Role>, guild_id: Snowflake) -> Result<(), CacheError>;
    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>, CacheError>;
    async fn delete_role(&self, id: Snowflake) -> Result<(), CacheError>;

    async fn store_emoji(&self, emoji: &Emoji, guild_id: Snowflake) -> Result<(), CacheError>;
    async fn store_emojis(&self, emojis: Vec<&Emoji>, guild_id: Snowflake) -> Result<(), CacheError>;
    async fn get_emoji(&self, emoji_id: Snowflake) -> Result<Option<Emoji>, CacheError>;
    async fn delete_emoji(&self, emoji_id: Snowflake) -> Result<(), CacheError>;

    async fn store_voice_state(&self, voice_state: &VoiceState) -> Result<(), CacheError>;
    async fn store_voice_states(&self, voice_states: Vec<&VoiceState>) -> Result<(), CacheError>;
    async fn get_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<VoiceState>, CacheError>;
    async fn delete_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<(), CacheError>;
}