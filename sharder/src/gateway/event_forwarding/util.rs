use crate::gateway::payloads::event::Event;
use model::Snowflake;

pub fn get_guild_id(event: &Event) -> Option<Snowflake> {
    match event {
        Event::ChannelCreate(data) => data.guild_id,
        Event::ChannelUpdate(data) => data.guild_id,
        Event::ChannelDelete(data) => data.guild_id,
        Event::ThreadCreate(data) => data.guild_id,
        Event::ThreadUpdate(data) => data.guild_id,
        Event::ThreadDelete(data) => Some(data.guild_id),
        Event::ThreadListSync(data) => Some(data.guild_id),
        Event::ThreadMembersUpdate(data) => Some(data.guild_id),
        Event::ChannelPinsUpdate(data) => data.guild_id,
        Event::GuildCreate(data) => Some(data.id),
        Event::GuildUpdate(data) => Some(data.id),
        Event::GuildDelete(data) => Some(data.id),
        Event::GuildBanAdd(data) => Some(data.guild_id),
        Event::GuildBanRemove(data) => Some(data.guild_id),
        Event::GuildEmojisUpdate(data) => Some(data.guild_id),
        Event::GuildIntegrationsUpdate(data) => Some(data.guild_id),
        Event::GuildMemberAdd(data) => Some(data.guild_id),
        Event::GuildMemberRemove(data) => Some(data.guild_id),
        Event::GuildMemberUpdate(data) => Some(data.guild_id),
        Event::GuildMembersChunk(data) => Some(data.guild_id),
        Event::GuildRoleCreate(data) => Some(data.guild_id),
        Event::GuildRoleUpdate(data) => Some(data.guild_id),
        Event::GuildRoleDelete(data) => Some(data.guild_id),
        Event::InviteCreate(data) => data.guild_id,
        Event::InviteDelete(data) => data.guild_id,
        Event::MessageCreate(data) => data.guild_id,
        Event::MessageUpdate(data) => data.guild_id,
        Event::MessageDelete(data) => data.guild_id,
        Event::MessageDeleteBulk(data) => data.guild_id,
        Event::MessageReactionAdd(data) => data.guild_id,
        Event::MessageReactionRemove(data) => data.guild_id,
        Event::MessageReactionRemoveAll(data) => data.guild_id,
        Event::MessageReactionRemoveEmoji(data) => data.guild_id,
        Event::PresenceUpdate(data) => data.guild_id,
        Event::TypingStart(data) => data.guild_id,
        Event::VoiceStateUpdate(data) => data.guild_id,
        Event::VoiceServerUpdate(data) => Some(data.guild_id),
        Event::WebhookUpdate(data) => Some(data.guild_id),
        _ => None,
    }
}

// TODO: Don't hardcode, use feature flags or something
pub fn is_whitelisted(event: &str) -> bool {
    matches!(
        event,
        // Cache events
        "CHANNEL_CREATE"
            | "CHANNEL_UPDATE"
            | "CHANNEL_DELETE"
            | "THREAD_CREATE"
            | "THREAD_UPDATE"
            | "THREAD_DELETE"
            | "GUILD_CREATE"
            | "GUILD_UPDATE"
            | "GUILD_DELETE"
            | "GUILD_BAN_ADD"
            | "GUILD_MEMBER_REMOVE"
            | "GUILD_MEMBER_UPDATE"
            | "GUILD_MEMBERS_CHUNK" // We never receive these
            | "GUILD_ROLE_CREATE"
            | "GUILD_ROLE_UPDATE"
            | "GUILD_ROLE_DELETE"
            | "USER_UPDATE"
            | "GUILD_EMOJIS_UPDATE"

            // Worker events
            | "MESSAGE_CREATE"
            | "THREAD_MEMBERS_UPDATE"
    )
}
