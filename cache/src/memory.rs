use crate::model::{CachedChannel, GuildState};
use crate::{Cache, Options, Result};
use async_trait::async_trait;
use dashmap::DashMap;
use model::channel::Channel;
use model::guild::{Emoji, Guild, Member, Role, VoiceState};
use model::user::User;
use model::Snowflake;

pub struct MemoryCache {
    opts: Options,
    guilds: DashMap<Snowflake, GuildState>,
    channels: DashMap<Snowflake, CachedChannel>,
}

impl MemoryCache {
    pub fn new(opts: Options) -> Self {
        MemoryCache {
            opts,
            guilds: DashMap::new(),
            channels: DashMap::new(),
        }
    }
}

#[async_trait]
impl Cache for MemoryCache {
    async fn store_guild(&self, guild: Guild) -> Result<()> {
        self.guilds.insert(guild.id, GuildState::from(guild));
        Ok(())
    }

    async fn store_guilds(&self, guilds: Vec<Guild>) -> Result<()> {
        guilds.into_iter().for_each(|g| {
            let _ = self.store_guild(g);
        });
        Ok(())
    }

    async fn get_guild(&self, id: Snowflake) -> Result<Option<Guild>> {
        todo!()
    }

    async fn delete_guild(&self, id: Snowflake) -> Result<()> {
        self.guilds.remove(&id);
        Ok(())
    }

    async fn get_guild_count(&self) -> Result<usize> {
        Ok(self.guilds.len())
    }

    async fn store_channel(&self, channel: Channel) -> Result<()> {
        self.channels
            .insert(channel.id, CachedChannel::from(channel));
        Ok(())
    }

    async fn store_channels(&self, channels: Vec<Channel>) -> Result<()> {
        self.channels.into_iter().for_each(|c| {
            let _ = self.store_channel(c);
        });
        Ok(())
    }

    async fn get_channel(&self, id: Snowflake) -> Result<Option<Channel>> {
        todo!()
    }

    async fn delete_channel(&self, id: Snowflake) -> Result<()> {
        todo!()
    }

    async fn store_user(&self, user: User) -> Result<()> {
        todo!()
    }

    async fn store_users(&self, users: Vec<User>) -> Result<()> {
        todo!()
    }

    async fn get_user(&self, id: Snowflake) -> Result<Option<User>> {
        todo!()
    }

    async fn delete_user(&self, id: Snowflake) -> Result<()> {
        todo!()
    }

    async fn store_member(&self, member: Member, guild_id: Snowflake) -> Result<()> {
        todo!()
    }

    async fn store_members(&self, members: Vec<Member>, guild_id: Snowflake) -> Result<()> {
        todo!()
    }

    async fn get_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<Member>> {
        todo!()
    }

    async fn delete_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        todo!()
    }

    async fn store_role(&self, role: Role, guild_id: Snowflake) -> Result<()> {
        todo!()
    }

    async fn store_roles(&self, roles: Vec<Role>, guild_id: Snowflake) -> Result<()> {
        todo!()
    }

    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>> {
        todo!()
    }

    async fn delete_role(&self, id: Snowflake) -> Result<()> {
        todo!()
    }

    async fn store_emoji(&self, emoji: Emoji, guild_id: Snowflake) -> Result<()> {
        todo!()
    }

    async fn store_emojis(&self, emojis: Vec<Emoji>, guild_id: Snowflake) -> Result<()> {
        todo!()
    }

    async fn get_emoji(&self, emoji_id: Snowflake) -> Result<Option<Emoji>> {
        todo!()
    }

    async fn delete_emoji(&self, emoji_id: Snowflake) -> Result<()> {
        todo!()
    }

    async fn store_voice_state(&self, voice_state: VoiceState) -> Result<()> {
        todo!()
    }

    async fn store_voice_states(&self, voice_states: Vec<VoiceState>) -> Result<()> {
        todo!()
    }

    async fn get_voice_state(
        &self,
        user_id: Snowflake,
        guild_id: Snowflake,
    ) -> Result<Option<VoiceState>> {
        todo!()
    }

    async fn delete_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        todo!()
    }
}
