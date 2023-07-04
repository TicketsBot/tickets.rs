use crate::CacheError;
use model::channel::Channel;
use model::guild::{Emoji, Guild, Member, Role, VoiceState};
use model::user::User;
use model::Snowflake;
use tokio::sync::oneshot;

type ResultSender<T> = oneshot::Sender<Result<T, CacheError>>;

#[derive(Debug)]
pub enum CachePayload {
    Schema {
        queries: Vec<String>,
    },

    StoreGuilds {
        guilds: Vec<Guild>,
    },
    GetGuild {
        id: Snowflake,
        tx: ResultSender<Option<Guild>>,
    },
    DeleteGuild {
        id: Snowflake,
    },
    GetGuildCount {
        tx: ResultSender<usize>,
    },

    StoreChannels {
        channels: Vec<Channel>,
    },
    GetChannel {
        id: Snowflake,
        tx: ResultSender<Option<Channel>>,
    },
    DeleteChannel {
        id: Snowflake,
    },

    StoreUsers {
        users: Vec<User>,
    },
    GetUser {
        id: Snowflake,
        tx: ResultSender<Option<User>>,
    },
    DeleteUser {
        id: Snowflake,
    },

    StoreMembers {
        members: Vec<Member>,
        guild_id: Snowflake,
    },
    GetMember {
        user_id: Snowflake,
        guild_id: Snowflake,
        tx: ResultSender<Option<Member>>,
    },
    DeleteMember {
        user_id: Snowflake,
        guild_id: Snowflake,
    },

    StoreRoles {
        roles: Vec<Role>,
        guild_id: Snowflake,
    },
    GetRole {
        id: Snowflake,
        tx: ResultSender<Option<Role>>,
    },
    DeleteRole {
        id: Snowflake,
    },

    StoreEmojis {
        emojis: Vec<Emoji>,
        guild_id: Snowflake,
    },
    GetEmoji {
        id: Snowflake,
        tx: ResultSender<Option<Emoji>>,
    },
    DeleteEmoji {
        id: Snowflake,
    },

    StoreVoiceState {
        voice_states: Vec<VoiceState>,
    },
    GetVoiceState {
        user_id: Snowflake,
        guild_id: Snowflake,
        tx: ResultSender<Option<VoiceState>>,
    },
    DeleteVoiceState {
        user_id: Snowflake,
        guild_id: Snowflake,
    },
}
