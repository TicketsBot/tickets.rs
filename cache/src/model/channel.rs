use chrono::{DateTime, Utc};
use model::channel::{
    Channel, ChannelType, PermissionOverwrite, ThreadMember, ThreadMetadata, VideoQualityMode,
};
use model::user::User;
use model::Snowflake;

#[derive(Debug)]
pub struct CachedChannel {
    pub channel_type: ChannelType,
    pub position: Option<u16>,
    pub permission_overwrites: Option<Vec<PermissionOverwrite>>,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub nsfw: Option<bool>,
    pub last_message_id: Option<Snowflake>,
    pub bitrate: Option<u32>,
    pub user_limit: Option<u16>,
    pub rate_limit_per_user: Option<u16>,
    pub recipients: Option<Vec<User>>,
    pub icon: Option<String>,
    pub owner_id: Option<Snowflake>,
    pub application_id: Option<Snowflake>,
    pub parent_id: Option<Snowflake>,
    pub last_pin_timestamp: Option<DateTime<Utc>>,
    pub rtc_region: Option<String>,
    pub video_quality_mode: Option<VideoQualityMode>,
    pub message_count: Option<usize>,
    pub member_count: Option<usize>,
    pub thread_metadata: Option<ThreadMetadata>,
    pub thread_member: Option<ThreadMember>,
}

impl From<CachedChannel> for Channel {
    fn from(_: CachedChannel) -> Self {
        todo!()
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
            recipients: other.recipients,
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
            thread_member: other.thread_member,
        }
    }
}
