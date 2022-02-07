use crate::model::{CachedChannel, CachedMember, CachedUser, ChannelMap, EntityMap, GuildState, UserMap};
use crate::{CacheError, Options, Result};
use dashmap::mapref::one::RefMut;
use dashmap::DashMap;
use model::channel::Channel;
use model::guild::{Emoji, Guild, Member, Role, VoiceState};
use model::user::User;
use model::Snowflake;

pub struct MemoryCache {
    opts: Options,
    guilds: DashMap<Snowflake, GuildState>,
    dm_channels: ChannelMap,
    users: UserMap,
}

impl MemoryCache {
    pub fn new(opts: Options) -> Self {
        MemoryCache {
            opts,
            guilds: DashMap::new(),
            dm_channels: ChannelMap::new(),
            users: UserMap::new(),
        }
    }

    pub fn guild_mut(&self, id: Snowflake) -> Option<RefMut<Snowflake, GuildState>> {
        self.guilds.get_mut(&id)
    }

    pub fn store_guild(&self, guild: Guild) -> Result<()> {
        if !self.opts.guilds {
            return Ok(());
        }

        self.guilds.insert(guild.id, GuildState::from(guild));
        Ok(())
    }

    pub fn store_guilds(&self, guilds: Vec<Guild>) -> Result<()> {
        if !self.opts.guilds {
            return Ok(());
        }

        guilds.into_iter().for_each(|g| {
            let _ = self.store_guild(g);
        });
        Ok(())
    }

    pub fn get_guild(&self, id: Snowflake, with: Options) -> Result<Option<Guild>> {
        if !self.opts.guilds {
            return CacheError::StoreDisabled.into();
        }

        let state = match self.guilds.get(&id) {
            Some(v) => v.value(),
            None => return Ok(None),
        };

        let roles = if with.roles && self.opts.roles {
            state
                .roles
                .iter()
                .map(|pair| pair.value().clone().into_role(*pair.key()))
                .collect()
        } else {
            vec![]
        };

        let emojis = if with.emojis && self.opts.emojis {
            state
                .emojis
                .iter()
                .map(|pair| pair.value().clone().into_emoji(*pair.key()))
                .collect()
        } else {
            vec![]
        };

        // Use with caution!
        let members = if with.members && self.opts.members && self.opts.users {
            state
                .members
                .iter()
                .filter_map(|pair| {
                    let id = *pair.key();
                    self.users
                        .get_converted(id)
                        .map(|user| pair.value().clone().into_member(user))
                })
                .collect()
        } else {
            vec![]
        };

        let channels = if with.channels && self.opts.channels {
            state
                .channels
                .iter()
                .map(|pair| pair.value().clone().into_channel(*pair.key(), Some(id)))
                .collect()
        } else {
            vec![]
        };

        let threads = if with.channels && self.opts.channels {
            state
                .threads
                .iter()
                .map(|pair| pair.value().clone().into_channel(*pair.key(), Some(id)))
                .collect()
        } else {
            vec![]
        };

        Ok(Some(state.guild.clone().into_guild_simple(
            id, roles, emojis, members, channels, threads,
        )))
    }

    pub fn delete_guild(&self, id: Snowflake) -> Result<()> {
        if !self.opts.guilds {
            return Ok(());
        }

        self.guilds.remove(&id);
        Ok(())
    }

    pub fn get_guild_count(&self) -> Result<usize> {
        Ok(self.guilds.len())
    }

    pub fn store_channel(&self, channel: Channel) -> Result<()> {
        if !self.opts.channels {
            return Ok(());
        }

        match channel.guild_id {
            Some(guild_id) => {
                if let Some(guild) = self.guild_mut(guild_id) {
                    guild
                        .channels
                        .insert(channel.id, CachedChannel::from(channel));
                }
            }
            None => {
                self.dm_channels
                    .insert(channel.id, CachedChannel::from(channel));
            }
        }

        Ok(())
    }

    pub fn store_channels(&self, channels: Vec<Channel>) -> Result<()> {
        if !self.opts.channels {
            return Ok(());
        }

        // Channels could be from different guilds, so we cannot save cycles by retrieving 1 guild
        channels.into_iter().for_each(|c| {
            let _ = self.store_channel(c);
        });
        Ok(())
    }

    pub fn get_channel(
        &self,
        id: Snowflake,
        guild_id: Option<Snowflake>,
    ) -> Result<Option<Channel>> {
        if !self.opts.channels {
            return CacheError::StoreDisabled.into();
        }

        match guild_id {
            Some(guild_id) => {
                if let Some(guild) = self.guild_mut(guild_id) {
                    Ok(guild.channels.get_converted(id, Some(guild_id)))
                } else {
                    CacheError::GuildNotFound(guild_id).into()
                }
            }
            None => Ok(self.dm_channels.get_converted(id, guild_id)),
        }
    }

    pub fn get_guild_channels(&self, guild_id: Snowflake) -> Result<Vec<Channel>> {
        if !self.opts.channels || !self.opts.guilds {
            return CacheError::StoreDisabled.into();
        }

        if let Some(guild) = self.guilds.get(&guild_id) {
            Ok(guild.channels.get_all_converted(guild_id))
        } else {
            CacheError::GuildNotFound(guild_id).into()
        }
    }

    pub fn delete_channel(&self, id: Snowflake, guild_id: Option<Snowflake>) -> Result<()> {
        match guild_id {
            Some(guild_id) => {
                self.guild_mut(guild_id)
                    .and_then(|guild| guild.channels.remove(&id));
            }
            None => {
                self.dm_channels.remove(&id);
            }
        }

        Ok(())
    }

    pub fn store_user(&self, user: User) -> Result<()> {
        if !self.opts.users {
            return Ok(());
        }

        self.users.insert(user.id, CachedUser::from(user));
        Ok(())
    }

    pub fn store_users(&self, users: Vec<User>) -> Result<()> {
        if !self.opts.users {
            return Ok(());
        }

        users.into_iter().for_each(|u| {
            let _ = self.store_user(u);
        });
        Ok(())
    }

    pub fn get_user(&self, id: Snowflake) -> Result<Option<User>> {
        if !self.opts.users {
            return CacheError::StoreDisabled.into();
        }

        Ok(self.users.get_converted(id))
    }

    pub fn delete_user(&self, id: Snowflake) -> Result<()> {
        if !self.opts.users {
            return Ok(());
        }

        self.users.remove(&id);
        Ok(())
    }

    pub fn store_member(&self, member: Member, guild_id: Snowflake) -> Result<()> {
        if !self.opts.members || !self.opts.users {
            return Ok(());
        }

        if member.user.is_none() {
            return CacheError::MemberMissingUser.into();
        }

        self.store_user(member.user.clone().unwrap())?;
        if let Some(guild) = self.guilds.get_mut(&guild_id) {
            guild
                .members
                .insert(member.user.as_ref().unwrap().id, CachedMember::from(member));
        }

        Ok(())
    }

    pub fn store_members(&self, members: Vec<Member>, guild_id: Snowflake) -> Result<()> {
        if !self.opts.members || !self.opts.users {
            return Ok(());
        }

        let guild = self.guild_mut(guild_id);

        members
            .into_iter()
            .filter(|m| m.user.is_some())
            .for_each(|m| {
                self.users.insert(
                    m.user.as_ref().unwrap().id,
                    CachedUser::from(m.user.clone().unwrap()),
                );

                guild.as_ref().and_then(|guild| {
                    guild
                        .members
                        .insert(m.user.as_ref().unwrap().id, CachedMember::from(m))
                });
            });

        Ok(())
    }

    pub fn get_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<Member>> {
        if !self.opts.members || !self.opts.users {
            return CacheError::StoreDisabled.into();
        }

        let user = match self.get_user(user_id)? {
            Some(u) => u,
            None => return Ok(None),
        };

        if let Some(guild) = self.guilds.get(&guild_id) {
            Ok(guild.members.get_converted(user))
        } else {
            Ok(None)
        }
    }

    pub fn delete_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        if !self.opts.members {
            return Ok(());
        }

        self.guild_mut(guild_id)
            .and_then(|guild| guild.members.remove(&user_id));
        Ok(())
    }

    pub fn store_role(&self, role: Role, guild_id: Snowflake) -> Result<()> {
        if !self.opts.roles {
            return Ok(());
        }

        if let Some(guild) = self.guild_mut(guild_id) {
            guild.roles.insert(role.id, role.into());
            Ok(())
        } else {
            CacheError::GuildNotFound(guild_id).into()
        }
    }

    pub fn store_roles(&self, roles: Vec<Role>, guild_id: Snowflake) -> Result<()> {
        if !self.opts.roles {
            return Ok(());
        }

        if let Some(guild) = self.guild_mut(guild_id) {
            roles.into_iter().for_each(|role| {
                guild.roles.insert(role.id, role.into());
            });
            Ok(())
        } else {
            CacheError::GuildNotFound(guild_id).into()
        }
    }

    pub fn get_role(&self, id: Snowflake, guild_id: Snowflake) -> Result<Option<Role>> {
        if !self.opts.roles {
            return CacheError::StoreDisabled.into();
        }

        let role = self
            .guilds
            .get(&guild_id)
            .map(|guild| guild.roles.get_converted(id))
            .flatten();

        Ok(role)
    }

    pub fn get_roles(&self, ids: Vec<Snowflake>, guild_id: Snowflake) -> Result<Vec<Role>> {
        if !self.opts.roles {
            return CacheError::StoreDisabled.into();
        }

        let guild = self
            .guilds
            .get(&guild_id)
            .ok_or_else(|| CacheError::GuildNotFound(guild_id))?
            .value();
        let roles = ids
            .iter()
            .filter_map(|id| guild.roles.get_converted(*id))
            .collect();

        Ok(roles)
    }

    pub fn get_guild_roles(&self, guild_id: Snowflake) -> Result<Vec<Role>> {
        if !self.opts.roles || !self.opts.guilds {
            return CacheError::StoreDisabled.into();
        }

        let roles = self
            .guilds
            .get(&guild_id)
            .ok_or_else(|| CacheError::GuildNotFound(guild_id))?
            .value()
            .roles
            .get_all_converted();

        Ok(roles)
    }

    pub fn delete_role(&self, id: Snowflake, guild_id: Snowflake) -> Result<()> {
        if !self.opts.roles {
            return Ok(());
        }

        self.guild_mut(guild_id)
            .map(|guild| guild.roles.remove(&id))
            .ok_or_else(|| CacheError::GuildNotFound(guild_id))?;

        Ok(())
    }

    pub fn store_emoji(&self, emoji: Emoji, guild_id: Snowflake) -> Result<()> {
        todo!()
    }

    pub fn store_emojis(&self, emojis: Vec<Emoji>, guild_id: Snowflake) -> Result<()> {
        todo!()
    }

    pub fn get_emoji(&self, emoji_id: Snowflake) -> Result<Option<Emoji>> {
        todo!()
    }

    pub fn delete_emoji(&self, emoji_id: Snowflake) -> Result<()> {
        todo!()
    }

    pub fn store_voice_state(&self, voice_state: VoiceState) -> Result<()> {
        todo!()
    }

    pub fn store_voice_states(&self, voice_states: Vec<VoiceState>) -> Result<()> {
        todo!()
    }

    pub fn get_voice_state(
        &self,
        user_id: Snowflake,
        guild_id: Snowflake,
    ) -> Result<Option<VoiceState>> {
        todo!()
    }

    pub fn delete_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        todo!()
    }
}
