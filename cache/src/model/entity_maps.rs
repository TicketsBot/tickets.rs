use crate::model::{CachedChannel, CachedEmoji, CachedMember, CachedRole, CachedUser};
use dashmap::DashMap;
use model::channel::Channel;
use model::guild::{Emoji, Member, Role};
use model::user::User;
use model::Snowflake;
use std::convert::TryFrom;
use std::iter::FromIterator;
use std::ops::Deref;

pub trait EntityMap {
    type Entity;
    type CachedEntity: Clone + TryFrom<Self::Entity>;

    fn new() -> Self;
    fn from_vec(entities: Vec<Self::Entity>) -> Self;
    fn from_vec_clone(entities: &Vec<Self::Entity>) -> Self;
}

// ========================
// RoleMap
// ========================
pub struct RoleMap(DashMap<Snowflake, CachedRole>);

impl RoleMap {
    pub fn get_converted(&self, id: Snowflake) -> Option<Role> {
        self.0
            .get(&id)
            .map(|role| role.value().clone().into_role(id))
    }

    pub fn get_all_converted(&self) -> Vec<Role> {
        self.0
            .iter()
            .map(|role| role.value().clone().into_role(*role.key()))
            .collect()
    }
}

impl EntityMap for RoleMap {
    type Entity = Role;
    type CachedEntity = CachedRole;

    fn new() -> Self {
        Self(DashMap::new())
    }

    fn from_vec(roles: Vec<Role>) -> Self {
        roles
            .into_iter()
            .map(|role| (role.id, CachedRole::from(role)))
            .collect()
    }

    fn from_vec_clone(roles: &Vec<Role>) -> Self {
        roles
            .iter()
            .map(|role| (role.id, CachedRole::from(role.clone())))
            .collect()
    }
}

impl Deref for RoleMap {
    type Target = DashMap<Snowflake, CachedRole>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromIterator<(Snowflake, CachedRole)> for RoleMap {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (Snowflake, CachedRole)>,
    {
        let map = DashMap::from_iter(iter);
        Self(map)
    }
}

// ========================
// ChannelMap
// ========================
pub struct ChannelMap(DashMap<Snowflake, CachedChannel>);

impl ChannelMap {
    pub fn get_converted(&self, id: Snowflake, guild_id: Option<Snowflake>) -> Option<Channel> {
        self.0
            .get(&id)
            .map(|channel| channel.value().clone().into_channel(id, guild_id))
    }

    pub fn get_all_converted(&self, guild_id: Snowflake) -> Vec<Channel> {
        self.0
            .iter()
            .map(|channel| channel.value().clone().into_channel(*channel.key(), Some(guild_id)))
            .collect()
    }
}

impl EntityMap for ChannelMap {
    type Entity = Channel;
    type CachedEntity = CachedChannel;

    fn new() -> Self {
        Self(DashMap::new())
    }

    fn from_vec(channels: Vec<Channel>) -> Self {
        channels
            .into_iter()
            .map(|channel| (channel.id, CachedChannel::from(channel)))
            .collect()
    }

    fn from_vec_clone(channels: &Vec<Channel>) -> Self {
        channels
            .iter()
            .map(|channel| (channel.id, CachedChannel::from(channel.clone())))
            .collect()
    }
}

impl Deref for ChannelMap {
    type Target = DashMap<Snowflake, CachedChannel>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromIterator<(Snowflake, CachedChannel)> for ChannelMap {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (Snowflake, CachedChannel)>,
    {
        let map = DashMap::from_iter(iter);
        Self(map)
    }
}

// ========================
// MemberMap
// ========================
pub struct MemberMap(DashMap<Snowflake, CachedMember>);

impl MemberMap {
    pub fn get_converted(&self, user: User) -> Option<Member> {
        self.0
            .get(&user.id)
            .map(|member| member.value().clone().into_member(user))
    }
}

impl EntityMap for MemberMap {
    type Entity = Member;
    type CachedEntity = CachedMember;

    fn new() -> Self {
        Self(DashMap::new())
    }

    fn from_vec(members: Vec<Member>) -> Self {
        members
            .into_iter()
            .filter(|member| member.user.is_some())
            .map(|member| {
                (
                    member.user.as_ref().unwrap().id,
                    Self::CachedEntity::from(member),
                )
            })
            .collect()
    }

    fn from_vec_clone(members: &Vec<Member>) -> Self {
        members
            .iter()
            .filter(|member| member.user.is_some())
            .map(|member| {
                (
                    member.user.as_ref().unwrap().id,
                    Self::CachedEntity::from(member.clone()),
                )
            })
            .collect()
    }
}

impl Deref for MemberMap {
    type Target = DashMap<Snowflake, CachedMember>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromIterator<(Snowflake, CachedMember)> for MemberMap {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (Snowflake, CachedMember)>,
    {
        let map = DashMap::from_iter(iter);
        Self(map)
    }
}

// ========================
// UserMap
// ========================
#[derive(Clone, Debug)]
pub struct UserMap(DashMap<Snowflake, CachedUser>);

impl UserMap {
    pub fn get_converted(&self, id: Snowflake) -> Option<User> {
        self.0
            .get(&id)
            .map(|user| user.value().clone().into_user(id))
    }
}

impl EntityMap for UserMap {
    type Entity = User;
    type CachedEntity = CachedUser;

    fn new() -> Self {
        Self(DashMap::new())
    }

    fn from_vec(users: Vec<User>) -> Self {
        users
            .into_iter()
            .map(|user| (user.id, Self::CachedEntity::from(user)))
            .collect()
    }

    fn from_vec_clone(users: &Vec<User>) -> Self {
        users
            .iter()
            .map(|user| (user.id, Self::CachedEntity::from(user.clone())))
            .collect()
    }
}

impl Deref for UserMap {
    type Target = DashMap<Snowflake, CachedUser>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromIterator<(Snowflake, CachedUser)> for UserMap {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (Snowflake, CachedUser)>,
    {
        let map = DashMap::from_iter(iter);
        Self(map)
    }
}

impl Into<Vec<User>> for UserMap {
    fn into(self) -> Vec<User> {
        let mut users = Vec::with_capacity(self.0.len());
        for (id, user) in self.0 {
            users.push(user.into_user(id));
        }
        users
    }
}

// user_id -> [guild_id]
pub struct UserIndex(DashMap<Snowflake, Vec<Snowflake>>);

impl UserIndex {
    pub fn new() -> Self {
        UserIndex(DashMap::new())
    }

    pub fn add_guild(&mut self, user_id: Snowflake, guild_id: Snowflake) {
        let mut guilds = self.0.entry(user_id).or_insert_with(Vec::new);
        guilds.push(guild_id);
    }

    pub fn get_guilds(&self, user_id: Snowflake) -> Option<&Vec<Snowflake>> {
        self.0.get(&user_id).map(|guilds| guilds.value())
    }

    // Returns new guilds length
    pub fn remove_guild(&mut self, user_id: Snowflake, guild_id: Snowflake) -> usize {
        let guilds = self.0.get_mut(&user_id);
        if let Some(mut guilds) = guilds {
            guilds.retain(|&guild| guild != guild_id);
            guilds.len()
        } else {
            0
        }
    }
}

// ========================
// EmojiMap
// ========================
pub struct EmojiMap(DashMap<Snowflake, CachedEmoji>);

impl EmojiMap {
    pub fn get_converted(&self, id: Snowflake) -> Option<Emoji> {
        self.0
            .get(&id)
            .map(|emoji| emoji.value().clone().into_emoji(id))
    }
}

impl EntityMap for EmojiMap {
    type Entity = Emoji;
    type CachedEntity = CachedEmoji;

    fn new() -> Self {
        Self(DashMap::new())
    }

    fn from_vec(emojis: Vec<Emoji>) -> Self {
        emojis
            .into_iter()
            .filter(|emoji| emoji.id.is_some())
            .filter_map(|emoji| {
                let id = emoji.id.unwrap();
                let emoji = Self::CachedEntity::try_from(emoji);
                match emoji {
                    Ok(emoji) => Some((id, emoji)),
                    Err(_) => None,
                }
            })
            .collect()
    }

    fn from_vec_clone(emojis: &Vec<Emoji>) -> Self {
        emojis
            .iter()
            .filter(|emoji| emoji.id.is_some())
            .filter_map(|emoji| {
                let id = emoji.id.unwrap();
                let emoji = Self::CachedEntity::try_from(emoji.clone());
                match emoji {
                    Ok(emoji) => Some((id, emoji)),
                    Err(_) => None,
                }
            })
            .collect()
    }
}

impl Deref for EmojiMap {
    type Target = DashMap<Snowflake, CachedEmoji>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromIterator<(Snowflake, CachedEmoji)> for EmojiMap {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (Snowflake, CachedEmoji)>,
    {
        let map = DashMap::from_iter(iter);
        Self(map)
    }
}
