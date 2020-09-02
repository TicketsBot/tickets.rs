use serde::Deserialize;

use crate::model::{Snowflake, ImageHash};
use crate::model::user::User;
use super::{ChannelType, PermissionOverwrite};

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Channel {
    GuildText(GuildTextChannel),
    DM(DMChannel),
    GuildVoice(GuildVoiceChannel),
    GroupDM(GroupDMChannel),
    GuildCategory(GuildCategoryChannel),
    GuildNews(GuildNewsChannel),
    GuildStore(GuildStoreChannel),
}

#[derive(Deserialize, Debug)]
pub struct GuildTextChannel {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub name: String,
    pub position: u16,
    pub permission_overwrites: Vec<PermissionOverwrite>,
    pub rate_limit_per_user: u16,
    pub nsfw: bool,
    pub topic: Option<String>,
    pub last_message_id: Option<Snowflake>,
    pub parent_id: Option<Snowflake>,
}

#[derive(Deserialize, Debug)]
pub struct DMChannel {
    pub id: Snowflake,
    pub last_message_id: Option<Snowflake>,
    pub recipients: Vec<User>,
}

#[derive(Deserialize, Debug)]
pub struct GuildVoiceChannel {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub name: String,
    pub nsfw: bool,
    pub position: u16,
    pub permission_overwrites: Vec<PermissionOverwrite>,
    pub bitrate: u32,
    pub user_limit: u8,
    pub parent_id: Option<Snowflake>,
}

#[derive(Deserialize, Debug)]
pub struct GroupDMChannel {
    pub id: Snowflake,
    pub name: String,
    pub icon: Option<ImageHash>,
    pub owner_id: Snowflake,
    pub last_message_id: Option<Snowflake>,
    pub recipients: Vec<User>,
}

#[derive(Deserialize, Debug)]
pub struct GuildCategoryChannel {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub name: String,
    pub parent_id: Option<Snowflake>,
    pub position: u16,
    pub nsfw: bool,
    pub permission_overwrites: Vec<PermissionOverwrite>,
}

#[derive(Deserialize, Debug)]
pub struct GuildNewsChannel {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub name: String,
    pub position: u16,
    pub permission_overwrites: Vec<PermissionOverwrite>,
    pub nsfw: bool,
    pub topic: Option<String>,
    pub last_message_id: Option<Snowflake>,
    pub parent_id: Option<Snowflake>,
}

#[derive(Deserialize, Debug)]
pub struct GuildStoreChannel {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub name: String,
    pub parent_id: Option<Snowflake>,
    pub position: u16,
    pub nsfw: bool,
    pub permission_overwrites: Vec<PermissionOverwrite>,
}