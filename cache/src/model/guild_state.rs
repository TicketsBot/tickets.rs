use super::CachedGuild;
use model::Snowflake;

pub struct GuildState {
    pub guild: CachedGuild,
    pub channel_ids: Vec<Snowflake>,
    pub thread_ids: Vec<Snowflake>,
    pub role_ids: Vec<Snowflake>,
    pub emoji_ids: Vec<Snowflake>,
    pub stage_instance_ids: Vec<Snowflake>,
    pub sticker_ids: Vec<Snowflake>,
}

impl From<model::guild::Guild> for GuildState {
    fn from(other: model::guild::Guild) -> Self {
        Self {
            channel_ids: other
                .channels
                .as_ref()
                .map(|channels| channels.iter().map(|c| c.id).collect())
                .unwrap_or_default(),
            thread_ids: other
                .threads
                .as_ref()
                .map(|threads| threads.iter().map(|t| t.id).collect())
                .unwrap_or_default(),
            role_ids: other.roles.iter().map(|r| r.id).collect(),
            emoji_ids: other.emojis.iter().filter_map(|e| e.id).collect(),
            stage_instance_ids: other
                .stage_instances
                .as_ref()
                .map(|stages| stages.iter().map(|s| s.id).collect())
                .unwrap_or_else(|| vec![]),
            sticker_ids: other
                .stickers
                .as_ref()
                .map(|stickers| stickers.iter().map(|s| s.id).collect())
                .unwrap_or_else(|| vec![]),
            guild: CachedGuild::from(other), // Must be last, as takes self
        }
    }
}
