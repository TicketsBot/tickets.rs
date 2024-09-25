use crate::{Cache, CacheError, CachePayload, Options, Result};
use model::user::User;
use model::Snowflake;

use async_trait::async_trait;

use model::channel::Channel;
use model::guild::{Emoji, Guild, Member, Role, VoiceState};
use tracing::{error, info, trace};

use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(feature = "metrics")]
use lazy_static::lazy_static;

use crate::postgres::worker::{PayloadReceiver, Worker};
#[cfg(feature = "metrics")]
use prometheus::{register_histogram_vec, HistogramVec};
use tokio::sync::{mpsc, oneshot};
use tokio_postgres::tls::NoTlsStream;
use tokio_postgres::{Connection, NoTls, Socket};

use backoff::ExponentialBackoff;

#[cfg(feature = "metrics")]
lazy_static! {
    static ref HISTOGRAM: HistogramVec =
        register_histogram_vec!("cache_timings", "Cache Timings", &["table"])
            .expect("Failed to register cache timings histogram");
}

pub struct PostgresCache {
    opts: Options,
    tx: mpsc::UnboundedSender<CachePayload>,
}

impl PostgresCache {
    /// panics if URI is invalid
    pub async fn connect(uri: String, opts: Options, workers: usize) -> Result<PostgresCache> {
        info!(worker_count = workers, options = ?opts, "Connecting to database");

        let (worker_tx, worker_rx) = mpsc::unbounded_channel();
        let worker_rx = Arc::new(Mutex::new(worker_rx));

        // start workers
        for id in 0..workers {
            let worker_rx = Arc::clone(&worker_rx);
            let uri = uri.clone();

            // run executor in background
            tokio::spawn(async move {
                // Loop to reconnect after conn dies
                loop {
                    let _: Result<()> =
                        backoff::future::retry(ExponentialBackoff::default(), || async {
                            info!(id, "Starting cache worker");
                            let (kill_tx, conn) = Self::spawn_worker(
                                id,
                                opts.clone(),
                                &uri[..],
                                Arc::clone(&worker_rx),
                            )
                            .await?;
                            info!(id, "Cache worker started and connected");

                            if let Err(e) = conn.await {
                                error!(id, error = %e, "Failed to connect to database");
                                return Err(backoff::Error::Transient(CacheError::DatabaseError(
                                    e,
                                )));
                            }

                            info!(id, "Cache worker disconnected");

                            kill_tx.send(());
                            Err(backoff::Error::Transient(CacheError::Disconnected))
                        })
                        .await;
                }
            });
        }

        Ok(PostgresCache {
            opts,
            tx: worker_tx,
        })
    }

    #[tracing::instrument(name = "spawn_worker", skip(uri, payload_rx))]
    async fn spawn_worker(
        id: usize,
        opts: Options,
        uri: &str,
        payload_rx: PayloadReceiver,
    ) -> Result<(oneshot::Sender<()>, Connection<Socket, NoTlsStream>)> {
        let (client, conn) = tokio_postgres::connect(uri, NoTls)
            .await
            .map_err(CacheError::DatabaseError)?;

        let (kill_tx, kill_rx) = oneshot::channel();

        let worker = Worker::new(id, opts, client, payload_rx, kill_rx);
        worker.start();

        Ok((kill_tx, conn))
    }

    pub async fn create_schema(&self) -> Result<()> {
        info!("Creating cache schema");

        let queries = vec![
            // create tables
            r#"SET synchronous_commit TO OFF;"#,
            r#"CREATE TABLE IF NOT EXISTS guilds("guild_id" int8 NOT NULL UNIQUE, "data" jsonb NOT NULL, PRIMARY KEY("guild_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS channels("channel_id" int8 NOT NULL UNIQUE, "guild_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("channel_id", "guild_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS users("user_id" int8 NOT NULL UNIQUE, "data" jsonb NOT NULL, "last_seen" TIMESTAMPTZ NOT NULL DEFAULT NOW(), PRIMARY KEY("user_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS members("guild_id" int8 NOT NULL, "user_id" int8 NOT NULL, "data" jsonb NOT NULL, "last_seen" TIMESTAMPTZ NOT NULL DEFAULT NOW(), PRIMARY KEY("guild_id", "user_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS roles("role_id" int8 NOT NULL UNIQUE, "guild_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("role_id", "guild_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS emojis("emoji_id" int8 NOT NULL UNIQUE, "guild_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("emoji_id", "guild_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS voice_states("guild_id" int8 NOT NULL, "user_id" INT8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("guild_id", "user_id"));"#,

            // create indexes
            // TODO: Cannot create index concurrently in transaction block
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS channels_guild_id ON channels("guild_id");"#,
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS members_guild_id ON members("guild_id");"#,
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS member_user_id ON members("user_id");"#,
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS roles_guild_id ON roles("guild_id");"#,
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS emojis_guild_id ON emojis("guild_id");"#,
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS voice_states_guild_id ON voice_states("guild_id");"#,
            r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS voice_states_user_id ON voice_states("user_id");"#,
        ].iter().map(|s| s.to_string()).collect();

        self.tx
            .clone()
            .send(CachePayload::Schema { queries })
            .map_err(CacheError::SendError)?;

        Ok(())
    }

    fn send_payload(&self, payload: CachePayload) -> Result<()> {
        trace!(payload = ?payload, "Sending cache payload to tx channel");
        self.tx.send(payload)?;
        Ok(())
    }

    async fn send_payload_and_listen<T>(
        &self,
        rx: oneshot::Receiver<Result<T>>,
        payload: CachePayload,
    ) -> Result<T> {
        trace!(payload = ?payload, "Sending cache payload to tx channel and waiting for response");
        self.tx.send(payload)?;
        rx.await?
    }
}

#[async_trait]
impl Cache for PostgresCache {
    #[tracing::instrument(name = "store_guild", skip(self, guild), fields(guild_id = %guild.id))]
    async fn store_guild(&self, guild: Guild) -> Result<()> {
        self.store_guilds(vec![guild]).await
    }

    #[tracing::instrument(name = "store_guilds", skip(self, guilds), fields(guild_count = guilds.len()))]
    async fn store_guilds(&self, mut guilds: Vec<Guild>) -> Result<()> {
        if guilds.is_empty() {
            return Ok(());
        }

        self.send_payload(CachePayload::StoreGuilds { guilds })
    }

    #[tracing::instrument(name = "get_guild", skip(self))]
    async fn get_guild(&self, id: Snowflake) -> Result<Option<Guild>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload_and_listen(rx, CachePayload::GetGuild { id, tx })
            .await
    }

    #[tracing::instrument(name = "delete_guild", skip(self))]
    async fn delete_guild(&self, id: Snowflake) -> Result<()> {
        self.send_payload(CachePayload::DeleteGuild { id })
    }

    #[tracing::instrument(name = "get_guild_count", skip(self))]
    async fn get_guild_count(&self) -> Result<usize> {
        let (tx, rx) = oneshot::channel();
        self.send_payload_and_listen(rx, CachePayload::GetGuildCount { tx })
            .await
    }

    #[tracing::instrument(name = "store_channel", skip(self, channel), fields(channel_id = %channel.id))]
    async fn store_channel(&self, channel: Channel) -> Result<()> {
        self.store_channels(vec![channel]).await
    }

    #[tracing::instrument(name = "store_channels", skip(self, channels), fields(channel_count = channels.len()))]
    async fn store_channels(&self, channels: Vec<Channel>) -> Result<()> {
        if !self.opts.channels {
            return Ok(());
        }

        self.send_payload(CachePayload::StoreChannels { channels })
    }

    #[tracing::instrument(name = "get_channel", skip(self))]
    async fn get_channel(&self, id: Snowflake) -> Result<Option<Channel>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload_and_listen(rx, CachePayload::GetChannel { id, tx })
            .await
    }

    #[tracing::instrument(name = "delete_channel", skip(self))]
    async fn delete_channel(&self, id: Snowflake) -> Result<()> {
        self.send_payload(CachePayload::DeleteChannel { id })
    }

    #[tracing::instrument(name = "store_user", skip(self, user), fields(user_id = %user.id))]
    async fn store_user(&self, user: User) -> Result<()> {
        self.store_users(vec![user]).await
    }

    #[tracing::instrument(name = "store_users", skip(self, users), fields(user_count = users.len()))]
    async fn store_users(&self, mut users: Vec<User>) -> Result<()> {
        if !self.opts.users {
            return Ok(());
        }

        self.send_payload(CachePayload::StoreUsers { users })
    }

    #[tracing::instrument(name = "get_user", skip(self))]
    async fn get_user(&self, id: Snowflake) -> Result<Option<User>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload_and_listen(rx, CachePayload::GetUser { id, tx })
            .await
    }

    #[tracing::instrument(name = "delete_user", skip(self))]
    async fn delete_user(&self, id: Snowflake) -> Result<()> {
        self.send_payload(CachePayload::DeleteUser { id })
    }

    #[tracing::instrument(name = "store_member", skip(self, member), fields(user_id = ?member.user.as_ref().map(|u| u.id)))]
    async fn store_member(&self, member: Member, guild_id: Snowflake) -> Result<()> {
        self.store_members(vec![member], guild_id).await
    }

    #[tracing::instrument(name = "store_members", skip(self, members), fields(member_count = members.len()))]
    async fn store_members(&self, members: Vec<Member>, guild_id: Snowflake) -> Result<()> {
        if !self.opts.members {
            return Ok(());
        }

        self.send_payload(CachePayload::StoreMembers { members, guild_id })
    }

    #[tracing::instrument(name = "get_member", skip(self))]
    async fn get_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<Member>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload_and_listen(
            rx,
            CachePayload::GetMember {
                user_id,
                guild_id,
                tx,
            },
        )
        .await
    }

    #[tracing::instrument(name = "delete_member", skip(self))]
    async fn delete_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        self.send_payload(CachePayload::DeleteMember { user_id, guild_id })
    }

    #[tracing::instrument(name = "store_role", skip(self, role), fields(role_id = %role.id))]
    async fn store_role(&self, role: Role, guild_id: Snowflake) -> Result<()> {
        self.store_roles(vec![role], guild_id).await
    }

    #[tracing::instrument(name = "store_roles", skip(self, roles), fields(role_count = roles.len()))]
    async fn store_roles(&self, mut roles: Vec<Role>, guild_id: Snowflake) -> Result<()> {
        if !self.opts.roles {
            return Ok(());
        }

        self.send_payload(CachePayload::StoreRoles { roles, guild_id })
    }

    #[tracing::instrument(name = "get_role", skip(self))]
    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload_and_listen(rx, CachePayload::GetRole { id, tx })
            .await
    }

    #[tracing::instrument(name = "delete_role", skip(self))]
    async fn delete_role(&self, id: Snowflake) -> Result<()> {
        self.send_payload(CachePayload::DeleteRole { id })
    }

    #[tracing::instrument(name = "store_emoji", skip(self, emoji), fields(emoji_id = ?emoji.id))]
    async fn store_emoji(&self, emoji: Emoji, guild_id: Snowflake) -> Result<()> {
        self.store_emojis(vec![emoji], guild_id).await
    }

    #[tracing::instrument(name = "store_emojis", skip(self, emojis), fields(emoji_count = emojis.len()))]
    async fn store_emojis(&self, emojis: Vec<Emoji>, guild_id: Snowflake) -> Result<()> {
        if !self.opts.emojis {
            return Ok(());
        }

        self.send_payload(CachePayload::StoreEmojis { emojis, guild_id })
    }

    #[tracing::instrument(name = "get_emoji", skip(self))]
    async fn get_emoji(&self, id: Snowflake) -> Result<Option<Emoji>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload_and_listen(rx, CachePayload::GetEmoji { id, tx })
            .await
    }

    #[tracing::instrument(name = "delete_emoji", skip(self))]
    async fn delete_emoji(&self, id: Snowflake) -> Result<()> {
        self.send_payload(CachePayload::DeleteEmoji { id })
    }

    #[tracing::instrument(name = "store_voice_state", skip(self, voice_state), fields(user_id = ?voice_state.user_id, guild_id = ?voice_state.guild_id))]
    async fn store_voice_state(&self, voice_state: VoiceState) -> Result<()> {
        self.store_voice_states(vec![voice_state]).await
    }

    #[tracing::instrument(name = "store_voice_states", skip(self, voice_states), fields(voice_state_count = voice_states.len()))]
    async fn store_voice_states(&self, voice_states: Vec<VoiceState>) -> Result<()> {
        if !self.opts.voice_states {
            return Ok(());
        }

        self.send_payload(CachePayload::StoreVoiceState { voice_states })
    }

    #[tracing::instrument(name = "get_voice_state", skip(self))]
    async fn get_voice_state(
        &self,
        user_id: Snowflake,
        guild_id: Snowflake,
    ) -> Result<Option<VoiceState>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload_and_listen(
            rx,
            CachePayload::GetVoiceState {
                user_id,
                guild_id,
                tx,
            },
        )
        .await
    }

    #[tracing::instrument(name = "delete_voice_state", skip(self))]
    async fn delete_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        self.send_payload(CachePayload::DeleteVoiceState { user_id, guild_id })
    }
}
