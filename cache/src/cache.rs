use super::Result;

use async_trait::async_trait;
use model::channel::Channel;
use model::guild::{Emoji, Guild, Member, Role, VoiceState};
use model::user::User;
use model::Snowflake;

#[async_trait]
pub trait Cache: Send + Sync + 'static {
    async fn store_guild(&self, guild: Guild) -> Result<()>;
    async fn store_guilds(&self, guilds: Vec<Guild>) -> Result<()>;
    async fn get_guild(&self, id: Snowflake) -> Result<Option<Guild>>;
    async fn delete_guild(&self, id: Snowflake) -> Result<()>;
    async fn get_guild_count(&self) -> Result<usize>;

    async fn store_channel(&self, channel: Channel) -> Result<()>;
    async fn store_channels(&self, channels: Vec<Channel>) -> Result<()>;
    async fn get_channel(&self, id: Snowflake) -> Result<Option<Channel>>;
    async fn delete_channel(&self, id: Snowflake) -> Result<()>;

    async fn store_user(&self, user: User) -> Result<()>;
    async fn store_users(&self, users: Vec<User>) -> Result<()>;
    async fn get_user(&self, id: Snowflake) -> Result<Option<User>>;
    async fn delete_user(&self, id: Snowflake) -> Result<()>;

    async fn store_member(&self, member: Member, guild_id: Snowflake) -> Result<()>;
    async fn store_members(&self, members: Vec<Member>, guild_id: Snowflake) -> Result<()>;
    async fn get_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<Member>>;
    async fn delete_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()>;

    async fn store_role(&self, role: Role, guild_id: Snowflake) -> Result<()>;
    async fn store_roles(&self, roles: Vec<Role>, guild_id: Snowflake) -> Result<()>;
    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>>;
    async fn delete_role(&self, id: Snowflake) -> Result<()>;

    async fn store_emoji(&self, emoji: Emoji, guild_id: Snowflake) -> Result<()>;
    async fn store_emojis(&self, emojis: Vec<Emoji>, guild_id: Snowflake) -> Result<()>;
    async fn get_emoji(&self, emoji_id: Snowflake) -> Result<Option<Emoji>>;
    async fn delete_emoji(&self, emoji_id: Snowflake) -> Result<()>;

    async fn store_voice_state(&self, voice_state: VoiceState) -> Result<()>;
    async fn store_voice_states(&self, voice_states: Vec<VoiceState>) -> Result<()>;
    async fn get_voice_state(
        &self,
        user_id: Snowflake,
        guild_id: Snowflake,
    ) -> Result<Option<VoiceState>>;
    async fn delete_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()>;
}
