use super::CachedGuild;
use crate::model::{ChannelMap, EmojiMap, EntityMap, MemberMap, RoleMap};
use model::guild::Guild;

pub struct GuildState {
    pub guild: CachedGuild,
    pub channels: ChannelMap,
    pub threads: ChannelMap,
    pub roles: RoleMap,
    pub members: MemberMap,
    pub emojis: EmojiMap,
    //pub stage_instance_ids: Vec<Snowflake>,
    //pub sticker_ids: Vec<Snowflake>,
}

impl From<Guild> for GuildState {
    fn from(other: Guild) -> Self {
        Self {
            // We can take options safely as CachedGuild doesn't store them
            channels: ChannelMap::from_vec(other.channels),
            threads: ChannelMap::from_vec(other.threads),
            roles: RoleMap::from_vec(other.roles),
            members: MemberMap::from_vec(other.members),
            emojis: EmojiMap::from_vec(other.emojis),
            // Inlined so that we don't have to clone roles, members, etc.
            guild: CachedGuild {
                name: other.name,
                icon: other.icon,
                splash: other.splash,
                discovery_splash: other.discovery_splash,
                owner_id: other.owner_id,
                afk_channel_id: other.afk_channel_id,
                afk_timeout: other.afk_timeout,
                widget_enabled: other.widget_enabled,
                widget_channel_id: other.widget_channel_id,
                verification_level: other.verification_level,
                default_message_notifications: other.default_message_notifications,
                explicit_content_filter: other.explicit_content_filter,
                features: other.features,
                mfa_level: other.mfa_level,
                application_id: other.application_id,
                system_channel_id: other.system_channel_id,
                system_channels_flags: other.system_channels_flags,
                rules_channel_id: other.rules_channel_id,
                joined_at: other.joined_at,
                large: other.large,
                unavailable: other.unavailable,
                member_count: other.member_count,
                max_presences: other.max_presences,
                max_members: other.max_members,
                vanity_url_code: other.vanity_url_code,
                description: other.description,
                banner: other.banner,
                premium_tier: other.premium_tier,
                premium_subscription_count: other.premium_subscription_count,
                preferred_locale: other.preferred_locale,
                public_updates_channel_id: other.public_updates_channel_id,
                max_video_channel_users: other.max_video_channel_users,
                approximate_member_count: other.approximate_member_count,
                approximate_presence_count: other.approximate_presence_count,
                welcome_screen: other.welcome_screen,
                nsfw_level: other.nsfw_level,
                premium_progress_bar_enabled: other.premium_progress_bar_enabled,
            },
        }
    }
}
