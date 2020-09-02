use serde::Deserialize;

use crate::model::channel::Channel;
use crate::model::guild::{Guild, UnavailableGuild, VoiceState};
use crate::model::channel::message::Message;
use crate::model::user::{PresenceUpdate, User};

#[derive(Deserialize, Debug)]
#[serde(tag = "t", content = "d")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Event {
    Ready(super::Ready),
    ChannelCreate(Channel),
    ChannelUpdate(Channel),
    ChannelDelete(Channel),
    ChannelPinsUpdate(super::ChannelPinsUpdate),
    GuildCreate(Guild),
    GuildUpdate(Guild),
    GuildDelete(UnavailableGuild),
    GuildBanAdd(super::GuildBanAdd),
    GuildBanRemove(super::GuildBanRemove),
    GuildEmojisUpdate(super::GuildEmojisUpdate),
    GuildIntegrationsUpdate(super::GuildIntegrationsUpdate),
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
    TypingStart(super::TypingStart),
    UserUpdate(User),
    VoiceStateUpdate(VoiceState),
    VoiceServerUpdate(super::VoiceServerUpdate),
    WebhooksUpdate(super::WebhooksUpdate),
}