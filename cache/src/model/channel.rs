use chrono::{DateTime, Utc};
use model::channel::{
    Channel, ChannelType, PermissionOverwrite, ThreadMember, ThreadMetadata, VideoQualityMode,
};
use model::Snowflake;
use crate::model::{UserMap, EntityMap};

#[derive(Clone, Debug)]
pub struct CachedChannel {
    pub channel_type: ChannelType,
    pub position: Option<u16>,
    pub permission_overwrites: Option<Vec<PermissionOverwrite>>,
    pub name: Box<str>,
    pub topic: Option<Box<str>>,
    pub nsfw: Option<bool>,
    pub last_message_id: Option<Snowflake>,
    pub bitrate: Option<u32>,
    pub user_limit: Option<u16>,
    pub rate_limit_per_user: Option<u16>,
    pub recipients: Option<UserMap>,
    pub icon: Option<Box<str>>, // TODO: ImageHash
    pub owner_id: Option<Snowflake>,
    pub application_id: Option<Snowflake>,
    pub parent_id: Option<Snowflake>,
    pub last_pin_timestamp: Option<DateTime<Utc>>,
    pub rtc_region: Option<Box<str>>,
    pub video_quality_mode: Option<VideoQualityMode>,
    pub message_count: Option<u8>,
    pub member_count: Option<u8>,
    pub thread_metadata: Option<ThreadMetadata>,
    pub member: Option<ThreadMember>,
}

impl CachedChannel {
    pub fn into_channel(self, id: Snowflake, guild_id: Option<Snowflake>) -> Channel {
        Channel {
            id,
            channel_type: self.channel_type,
            guild_id,
            position: self.position,
            permission_overwrites: self.permission_overwrites,
            name: self.name,
            topic: self.topic,
            nsfw: self.nsfw,
            last_message_id: self.last_message_id,
            bitrate: self.bitrate,
            user_limit: self.user_limit,
            rate_limit_per_user: self.rate_limit_per_user,
            recipients: self.recipients.map(|recipients| recipients.into()),
            icon: self.icon,
            owner_id: self.owner_id,
            application_id: self.application_id,
            parent_id: self.parent_id,
            last_pin_timestamp: self.last_pin_timestamp,
            rtc_region: self.rtc_region,
            video_quality_mode: self.video_quality_mode,
            message_count: self.message_count,
            member_count: self.member_count,
            thread_metadata: self.thread_metadata,
            member: self.member,
        }
    }
}

impl From<Channel> for CachedChannel {
    fn from(other: Channel) -> Self {
        Self {
            channel_type: other.channel_type,
            position: other.position,
            permission_overwrites: other.permission_overwrites,
            name: other.name,
            topic: other.topic,
            nsfw: other.nsfw,
            last_message_id: other.last_message_id,
            bitrate: other.bitrate,
            user_limit: other.user_limit,
            rate_limit_per_user: other.rate_limit_per_user,
            recipients: other.recipients.map(UserMap::from_vec),
            icon: other.icon,
            owner_id: other.owner_id,
            application_id: other.application_id,
            parent_id: other.parent_id,
            last_pin_timestamp: other.last_pin_timestamp,
            rtc_region: other.rtc_region,
            video_quality_mode: other.video_quality_mode,
            message_count: other.message_count,
            member_count: other.member_count,
            thread_metadata: other.thread_metadata,
            member: other.member,
        }
    }
}
