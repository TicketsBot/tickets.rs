use core::fmt;

use serde::{Deserialize, Serialize};

use model::channel::message::Message;
use model::channel::{Channel, ThreadMember};
use model::guild::{Guild, UnavailableGuild, VoiceState};
use model::interaction::{ApplicationCommand, GuildApplicationCommandPermissions};
use model::stage::StageInstance;
use model::user::{PresenceUpdate, User};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "t", content = "d")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Event {
    Ready(super::Ready),
    Resumed(serde_json::Value),
    ApplicationCommandCreate(ApplicationCommand),
    ApplicationCommandUpdate(ApplicationCommand),
    ApplicationCommandDelete(ApplicationCommand),
    ApplicationCommandPermissionsUpdate(GuildApplicationCommandPermissions),
    ChannelCreate(Channel),
    ChannelUpdate(Channel),
    ChannelDelete(Channel),
    ChannelPinsUpdate(super::ChannelPinsUpdate),
    ThreadCreate(Channel),
    ThreadUpdate(Channel),
    ThreadDelete(super::ThreadDelete),
    ThreadListSync(super::ThreadListSync),
    ThreadMemberUpdate(ThreadMember),
    ThreadMembersUpdate(super::ThreadMembersUpdate),
    GuildCreate(Guild),
    GuildUpdate(Guild),
    GuildDelete(UnavailableGuild),
    GuildBanAdd(super::GuildBanAdd),
    GuildBanRemove(super::GuildBanRemove),
    GuildEmojisUpdate(super::GuildEmojisUpdate),
    GuildIntegrationsUpdate(super::GuildIntegrationsUpdate),
    GuildJoinRequestUpdate(super::GuildJoinRequestUpdate),
    GuildJoinRequestDelete(super::GuildJoinRequestDelete),
    GuildMemberAdd(super::GuildMemberAdd),
    GuildMemberRemove(super::GuildMemberRemove),
    GuildMemberUpdate(super::GuildMemberUpdate),
    GuildMembersChunk(super::GuildMembersChunk),
    GuildRoleCreate(super::GuildRoleCreate),
    GuildRoleUpdate(super::GuildRoleUpdate),
    GuildRoleDelete(super::GuildRoleDelete),
    InviteCreate(super::InviteCreate),
    InviteDelete(super::InviteDelete),
    MessageCreate(Message),
    MessageUpdate(Message),
    MessageDelete(super::MessageDelete),
    MessageDeleteBulk(super::MessageDeleteBulk),
    MessageReactionAdd(super::MessageReactionAdd),
    MessageReactionRemove(super::MessageReactionRemove),
    MessageReactionRemoveAll(super::MessageReactionRemoveAll),
    MessageReactionRemoveEmoji(super::MessageReactionRemoveEmoji),
    PresenceUpdate(PresenceUpdate),
    StageInstanceCreate(StageInstance),
    StageInstanceUpdate(StageInstance),
    StageInstanceDelete(StageInstance),
    TypingStart(super::TypingStart),
    UserUpdate(User),
    VoiceChannelStatusUpdate(super::VoiceChannelStatusUpdate),
    VoiceStateUpdate(VoiceState),
    VoiceServerUpdate(super::VoiceServerUpdate),
    WebhookUpdate(super::WebhookUpdate),
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Event::Ready(_) => write!(f, "READY"),
            Event::Resumed(_) => write!(f, "RESUMED"),
            Event::ApplicationCommandCreate(_) => write!(f, "APPLICATION_COMMAND_CREATE"),
            Event::ApplicationCommandUpdate(_) => write!(f, "APPLICATION_COMMAND_UPDATE"),
            Event::ApplicationCommandDelete(_) => write!(f, "APPLICATION_COMMAND_DELETE"),
            Event::ApplicationCommandPermissionsUpdate(_) => {
                write!(f, "APPLICATION_COMMAND_PERMISSIONS_UPDATE")
            }
            Event::ChannelCreate(_) => write!(f, "CHANNEL_CREATE"),
            Event::ChannelUpdate(_) => write!(f, "CHANNEL_UPDATE"),
            Event::ChannelDelete(_) => write!(f, "CHANNEL_DELETE"),
            Event::ChannelPinsUpdate(_) => write!(f, "CHANNEL_PINS_UPDATE"),
            Event::ThreadCreate(_) => write!(f, "THREAD_CREATE"),
            Event::ThreadUpdate(_) => write!(f, "THREAD_UPDATE"),
            Event::ThreadDelete(_) => write!(f, "THREAD_DELETE"),
            Event::ThreadListSync(_) => write!(f, "THREAD_LIST_SYNC"),
            Event::ThreadMemberUpdate(_) => write!(f, "THREAD_MEMBER_UPDATE"),
            Event::ThreadMembersUpdate(_) => write!(f, "THREAD_MEMBERS_UPDATE"),
            Event::GuildCreate(_) => write!(f, "GUILD_CREATE"),
            Event::GuildUpdate(_) => write!(f, "GUILD_UPDATE"),
            Event::GuildDelete(_) => write!(f, "GUILD_DELETE"),
            Event::GuildBanAdd(_) => write!(f, "GUILD_BAN_ADD"),
            Event::GuildBanRemove(_) => write!(f, "GUILD_BAN_REMOVE"),
            Event::GuildEmojisUpdate(_) => write!(f, "GUILD_EMOJIS_UPDATE"),
            Event::GuildIntegrationsUpdate(_) => write!(f, "GUILD_INTEGRATIONS_UPDATE"),
            Event::GuildJoinRequestUpdate(_) => write!(f, "GUILD_JOIN_REQUEST_UPDATE"),
            Event::GuildJoinRequestDelete(_) => write!(f, "GUILD_JOIN_REQUEST_DELETE"),
            Event::GuildMemberAdd(_) => write!(f, "GUILD_MEMBER_ADD"),
            Event::GuildMemberRemove(_) => write!(f, "GUILD_MEMBER_REMOVE"),
            Event::GuildMemberUpdate(_) => write!(f, "GUILD_MEMBER_UPDATE"),
            Event::GuildMembersChunk(_) => write!(f, "GUILD_MEMBERS_CHUNK"),
            Event::GuildRoleCreate(_) => write!(f, "GUILD_ROLE_CREATE"),
            Event::GuildRoleUpdate(_) => write!(f, "GUILD_ROLE_UPDATE"),
            Event::GuildRoleDelete(_) => write!(f, "GUILD_ROLE_DELETE"),
            Event::InviteCreate(_) => write!(f, "INVITE_CREATE"),
            Event::InviteDelete(_) => write!(f, "INVITE_DELETE"),
            Event::MessageCreate(_) => write!(f, "MESSAGE_CREATE"),
            Event::MessageUpdate(_) => write!(f, "MESSAGE_UPDATE"),
            Event::MessageDelete(_) => write!(f, "MESSAGE_DELETE"),
            Event::MessageDeleteBulk(_) => write!(f, "MESSAGE_DELETE_BULK"),
            Event::MessageReactionAdd(_) => write!(f, "MESSAGE_REACTION_ADD"),
            Event::MessageReactionRemove(_) => write!(f, "MESSAGE_REACTION_REMOVE"),
            Event::MessageReactionRemoveAll(_) => write!(f, "MESSAGE_REACTION_REMOVE_ALL"),
            Event::MessageReactionRemoveEmoji(_) => write!(f, "MESSAGE_REACTION_REMOVE_EMOJI"),
            Event::PresenceUpdate(_) => write!(f, "PRESENCE_UPDATE"),
            Event::StageInstanceCreate(_) => write!(f, "STAGE_INSTANCE_CREATE"),
            Event::StageInstanceUpdate(_) => write!(f, "STAGE_INSTANCE_UPDATE"),
            Event::StageInstanceDelete(_) => write!(f, "STAGE_INSTANCE_DELETE"),
            Event::TypingStart(_) => write!(f, "TYPING_START"),
            Event::UserUpdate(_) => write!(f, "USER_UPDATE"),
            Event::VoiceChannelStatusUpdate(_) => write!(f, "VOICE_CHANNEL_STATUS_UPDATE"),
            Event::VoiceStateUpdate(_) => write!(f, "VOICE_STATE_UPDATE"),
            Event::VoiceServerUpdate(_) => write!(f, "VOICE_SERVER_UPDATE"),
            Event::WebhookUpdate(_) => write!(f, "WEBHOOK_UPDATE"),
        }
    }
}
