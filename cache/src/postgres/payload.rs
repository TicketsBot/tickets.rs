use model::guild::{Guild, Member, Role, Emoji, VoiceState};
use tokio::sync::oneshot;
use crate::CacheError;
use model::Snowflake;
use model::channel::Channel;
use model::user::User;

type ResultSender<T> = oneshot::Sender<Result<T, CacheError>>;

#[derive(Debug)]
pub enum CachePayload {
    Schema { queries: Vec<String>, tx: ResultSender<()> },

    StoreGuilds { guilds: Vec<Guild>, tx: ResultSender<()> },
    GetGuild { id: Snowflake, tx: ResultSender<Option<Guild>> },
    DeleteGuild { id: Snowflake, tx: ResultSender<()> },

    StoreChannels { channels: Vec<Channel>, tx: ResultSender<()> },
    GetChannel { id: Snowflake, tx: ResultSender<Option<Channel>> },
    DeleteChannel { id: Snowflake, tx: ResultSender<()> },
    
    StoreUsers { users: Vec<User>, tx: ResultSender<()> },
    GetUser { id: Snowflake, tx: ResultSender<Option<User>> },
    DeleteUser { id: Snowflake, tx: ResultSender<()> },

    StoreMembers { members: Vec<Member>, guild_id: Snowflake, tx: ResultSender<()> },
    GetMember { user_id: Snowflake, guild_id: Snowflake, tx: ResultSender<Option<Member>> },
    DeleteMember { user_id: Snowflake, guild_id: Snowflake, tx: ResultSender<()> },

    StoreRoles { roles: Vec<Role>, guild_id: Snowflake, tx: ResultSender<()> },
    GetRole { id: Snowflake, tx: ResultSender<Option<Role>> },
    DeleteRole { id: Snowflake, tx: ResultSender<()> },

    StoreEmojis { emojis: Vec<Emoji>, guild_id: Snowflake, tx: ResultSender<()> },
    GetEmoji { id: Snowflake, tx: ResultSender<Option<Emoji>> },
    DeleteEmoji { id: Snowflake, tx: ResultSender<()> },

    StoreVoiceState { voice_states: Vec<VoiceState>, tx: ResultSender<()> },
    GetVoiceState { user_id: Snowflake, guild_id: Snowflake, tx: ResultSender<Option<VoiceState>> },
    DeleteVoiceState { user_id: Snowflake, guild_id: Snowflake, tx: ResultSender<()> },
}