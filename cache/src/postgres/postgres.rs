use model::user::User;
use model::Snowflake;
use crate::{CacheError, Options, Cache};

use async_trait::async_trait;

use model::channel::Channel;
use model::guild::{Role, Guild, Member, Emoji, VoiceState};
use tokio::sync::{mpsc, oneshot, Mutex};
use std::sync::Arc;
use crate::postgres::payload::CachePayload;
use crate::postgres::worker::Worker;
use tokio_postgres::NoTls;

pub struct PostgresCache {
    opts: Options,
    tx: mpsc::Sender<CachePayload>,
}

impl PostgresCache {
    /// panics if URI is invalid
    pub async fn connect(uri: &str, opts: Options, workers: usize) -> Result<PostgresCache, CacheError> {
        let (worker_tx, worker_rx) = mpsc::channel(1); // TODO: Tweak
        let worker_rx = Arc::new(Mutex::new(worker_rx));

        // start workers
        for _ in 0..workers {
            let (client, conn) = tokio_postgres::connect(uri, NoTls).await.map_err(CacheError::DatabaseError)?;

            // run executor in background
            tokio::spawn(async move {
               if let Err(e) = conn.await {
                   panic!(e); // TODO: should we panic or not?
               }
            });

            let worker = Worker::new(client, Arc::clone(&worker_rx));
            worker.start();
        }

        Ok(PostgresCache {
            opts,
            tx: worker_tx,
        })
    }

    pub async fn create_schema(&self) -> Result<(), CacheError> {
        let queries = vec![
            // create tables
            r#"SET synchronous_commit TO OFF"#,
            r#"CREATE TABLE IF NOT EXISTS guilds("guild_id" int8 NOT NULL UNIQUE, "data" jsonb NOT NULL, PRIMARY KEY("guild_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS channels("channel_id" int8 NOT NULL UNIQUE, "guild_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("channel_id", "guild_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS users("user_id" int8 NOT NULL UNIQUE, "data" jsonb NOT NULL, PRIMARY KEY("user_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS members("guild_id" int8 NOT NULL, "user_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("guild_id", "user_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS roles("role_id" int8 NOT NULL UNIQUE, "guild_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("role_id", "guild_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS emojis("emoji_id" int8 NOT NULL UNIQUE, "guild_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("emoji_id", "guild_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS voice_states("guild_id" int8 NOT NULL, "user_id" INT8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("guild_id", "user_id"));"#,

            // create indexes
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS channels_guild_id ON channels("guild_id");"#,
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS members_guild_id ON members("guild_id");"#,
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS member_user_id ON members("user_id");"#,
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS roles_guild_id ON roles("guild_id");"#,
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS emojis_guild_id ON emojis("guild_id");"#,
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS voice_states_guild_id ON voice_states("guild_id");"#,
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS voice_states_user_id ON voice_states("user_id");"#,
        ].iter().map(|s| s.to_string()).collect();

        let (tx, rx) = oneshot::channel();
        self.tx.clone().send(CachePayload::Schema { queries, tx, }).await.map_err(CacheError::SendError)?;
        rx.await.map_err(CacheError::RecvError)??;

        Ok(())
    }

    async fn send_payload<T>(&self, rx: oneshot::Receiver<Result<T, CacheError>>, payload: CachePayload) -> Result<T, CacheError> {
        self.tx.clone().send(payload).await.map_err(CacheError::SendError)?;
        rx.await.map_err(CacheError::RecvError)?
    }
}

#[async_trait]
impl Cache for PostgresCache {
    async fn store_guild(&self, guild: Guild) -> Result<(), CacheError> {
        self.store_guilds(vec![guild]).await
    }

    async fn store_guilds(&self, guilds: Vec<Guild>) -> Result<(), CacheError> {
        if !self.opts.guilds {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::StoreGuilds { guilds, tx }).await
    }

    async fn get_guild(&self, id: Snowflake) -> Result<Option<Guild>, CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetGuild { id, tx }).await
    }

    async fn delete_guild(&self, id: Snowflake) -> Result<(), CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::DeleteGuild { id, tx }).await
    }

    async fn store_channel(&self, channel: Channel) -> Result<(), CacheError> {
        self.store_channels(vec![channel]).await
    }

    async fn store_channels(&self, channels: Vec<Channel>) -> Result<(), CacheError> {
        if !self.opts.channels {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::StoreChannels { channels, tx }).await
    }

    async fn get_channel(&self, id: Snowflake) -> Result<Option<Channel>, CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetChannel { id, tx }).await
    }

    async fn delete_channel(&self, id: Snowflake) -> Result<(), CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::DeleteChannel { id, tx }).await
    }

    async fn store_user(&self, user: User) -> Result<(), CacheError> {
        self.store_users(vec![user]).await
    }

    async fn store_users(&self, users: Vec<User>) -> Result<(), CacheError> {
        if !self.opts.users {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::StoreUsers { users, tx }).await
    }

    async fn get_user(&self, id: Snowflake) -> Result<Option<User>, CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetUser { id, tx }).await
    }

    async fn delete_user(&self, id: Snowflake) -> Result<(), CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::DeleteUser { id, tx }).await
    }

    async fn store_member(&self, member: Member, guild_id: Snowflake) -> Result<(), CacheError> {
        self.store_members(vec![member], guild_id).await
    }

    async fn store_members(&self, members: Vec<Member>, guild_id: Snowflake) -> Result<(), CacheError> {
        if !self.opts.members {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::StoreMembers{ members, guild_id, tx }).await
    }

    async fn get_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<Member>, CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetMember { user_id, guild_id, tx }).await
    }

    async fn delete_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<(), CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::DeleteMember { user_id, guild_id, tx }).await
    }

    async fn store_role(&self, role: Role, guild_id: Snowflake) -> Result<(), CacheError> {
        self.store_roles(vec![role], guild_id).await
    }

    async fn store_roles(&self, roles: Vec<Role>, guild_id: Snowflake) -> Result<(), CacheError> {
        if !self.opts.roles {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::StoreRoles { roles, guild_id, tx }).await
    }

    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>, CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetRole { id, tx }).await
    }

    async fn delete_role(&self, id: Snowflake) -> Result<(), CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::DeleteRole { id, tx }).await
    }

    async fn store_emoji(&self, emoji: Emoji, guild_id: Snowflake) -> Result<(), CacheError> {
        self.store_emojis(vec![emoji], guild_id).await
    }

    async fn store_emojis(&self, emojis: Vec<Emoji>, guild_id: Snowflake) -> Result<(), CacheError> {
        if !self.opts.emojis {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::StoreEmojis { emojis, guild_id, tx }).await
    }

    async fn get_emoji(&self, id: Snowflake) -> Result<Option<Emoji>, CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetEmoji { id, tx }).await
    }

    async fn delete_emoji(&self, id: Snowflake) -> Result<(), CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::DeleteEmoji { id, tx }).await
    }

    async fn store_voice_state(&self, voice_state: VoiceState) -> Result<(), CacheError> {
        self.store_voice_states(vec![voice_state]).await
    }

    async fn store_voice_states(&self, voice_states: Vec<VoiceState>) -> Result<(), CacheError> {
        if !self.opts.voice_states {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::StoreVoiceState { voice_states, tx }).await
    }

    async fn get_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<VoiceState>, CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetVoiceState { user_id, guild_id, tx }).await
    }

    async fn delete_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<(), CacheError> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::DeleteVoiceState { user_id, guild_id, tx }).await
    }
}
