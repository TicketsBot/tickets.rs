use crate::{Cache, CacheError, Options, Result};
use model::user::User;
use model::Snowflake;

use async_trait::async_trait;

use crate::postgres::payload::CachePayload;
use crate::postgres::worker::{PayloadReceiver, Worker};
use backoff::ExponentialBackoff;
use model::channel::Channel;
use model::guild::{Emoji, Guild, Member, Role, VoiceState};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_postgres::tls::NoTlsStream;
use tokio_postgres::{Connection, NoTls, Socket};

pub struct PostgresCache {
    opts: Options,
    tx: mpsc::Sender<CachePayload>,
}

impl PostgresCache {
    /// panics if URI is invalid
    pub async fn connect(uri: String, opts: Options, workers: usize) -> Result<PostgresCache> {
        let (worker_tx, worker_rx) = mpsc::channel(1); // TODO: Tweak
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
                            println!("[cache worker:{}] trying to connect", id);
                            let (kill_tx, conn) =
                                Self::spawn_worker(id, &uri[..], Arc::clone(&worker_rx)).await?;
                            println!("[cache worker:{}] connected!", id);

                            if let Err(e) = conn.await {
                                eprintln!("[cache worker:{}] db connection error: {}", id, e);
                                return Err(backoff::Error::Transient(CacheError::DatabaseError(
                                    e,
                                )));
                            }

                            println!("[cache worker:{}] connection died", id);

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

    async fn spawn_worker(
        id: usize,
        uri: &str,
        payload_rx: PayloadReceiver,
    ) -> Result<(oneshot::Sender<()>, Connection<Socket, NoTlsStream>)> {
        let (client, conn) = tokio_postgres::connect(uri, NoTls)
            .await
            .map_err(CacheError::DatabaseError)?;

        let (kill_tx, kill_rx) = oneshot::channel();

        let worker = Worker::new(id, client, payload_rx, kill_rx);
        worker.start();

        Ok((kill_tx, conn))
    }

    pub async fn create_schema(&self) -> Result<()> {
        let queries = vec![
            // create tables
            r#"SET synchronous_commit TO OFF;"#,
            r#"CREATE TABLE IF NOT EXISTS guilds("guild_id" int8 NOT NULL UNIQUE, "data" jsonb NOT NULL, PRIMARY KEY("guild_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS channels("channel_id" int8 NOT NULL UNIQUE, "guild_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("channel_id", "guild_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS users("user_id" int8 NOT NULL UNIQUE, "data" jsonb NOT NULL, PRIMARY KEY("user_id"));"#,
            r#"CREATE TABLE IF NOT EXISTS members("guild_id" int8 NOT NULL, "user_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("guild_id", "user_id"));"#,
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

        let (tx, rx) = oneshot::channel();
        self.tx
            .clone()
            .send(CachePayload::Schema { queries, tx })
            .await
            .map_err(CacheError::SendError)?;
        rx.await.map_err(CacheError::RecvError)??;

        Ok(())
    }

    async fn send_payload<T>(
        &self,
        rx: oneshot::Receiver<Result<T>>,
        payload: CachePayload,
    ) -> Result<T> {
        self.tx
            .clone()
            .send(payload)
            .await
            .map_err(CacheError::SendError)?;
        rx.await.map_err(CacheError::RecvError)?
    }
}

#[async_trait]
impl Cache for PostgresCache {
    async fn store_guild(&self, guild: Guild) -> Result<()> {
        self.store_guilds(vec![guild]).await
    }

    async fn store_guilds(&self, guilds: Vec<Guild>) -> Result<()> {
        if !self.opts.guilds {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::StoreGuilds { guilds, tx })
            .await
    }

    async fn get_guild(&self, id: Snowflake) -> Result<Option<Guild>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetGuild { id, tx })
            .await
    }

    async fn delete_guild(&self, id: Snowflake) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::DeleteGuild { id, tx })
            .await
    }

    async fn get_guild_count(&self) -> Result<usize> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetGuildCount { tx })
            .await
    }

    async fn store_channel(&self, channel: Channel) -> Result<()> {
        self.store_channels(vec![channel]).await
    }

    async fn store_channels(&self, channels: Vec<Channel>) -> Result<()> {
        if !self.opts.channels {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::StoreChannels { channels, tx })
            .await
    }

    async fn get_channel(&self, id: Snowflake) -> Result<Option<Channel>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetChannel { id, tx })
            .await
    }

    async fn delete_channel(&self, id: Snowflake) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::DeleteChannel { id, tx })
            .await
    }

    async fn store_user(&self, user: User) -> Result<()> {
        self.store_users(vec![user]).await
    }

    async fn store_users(&self, users: Vec<User>) -> Result<()> {
        if !self.opts.users {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::StoreUsers { users, tx })
            .await
    }

    async fn get_user(&self, id: Snowflake) -> Result<Option<User>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetUser { id, tx })
            .await
    }

    async fn delete_user(&self, id: Snowflake) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::DeleteUser { id, tx })
            .await
    }

    async fn store_member(&self, member: Member, guild_id: Snowflake) -> Result<()> {
        self.store_members(vec![member], guild_id).await
    }

    async fn store_members(&self, members: Vec<Member>, guild_id: Snowflake) -> Result<()> {
        if !self.opts.members {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(
            rx,
            CachePayload::StoreMembers {
                members,
                guild_id,
                tx,
            },
        )
        .await
    }

    async fn get_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<Member>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(
            rx,
            CachePayload::GetMember {
                user_id,
                guild_id,
                tx,
            },
        )
        .await
    }

    async fn delete_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(
            rx,
            CachePayload::DeleteMember {
                user_id,
                guild_id,
                tx,
            },
        )
        .await
    }

    async fn store_role(&self, role: Role, guild_id: Snowflake) -> Result<()> {
        self.store_roles(vec![role], guild_id).await
    }

    async fn store_roles(&self, roles: Vec<Role>, guild_id: Snowflake) -> Result<()> {
        if !self.opts.roles {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(
            rx,
            CachePayload::StoreRoles {
                roles,
                guild_id,
                tx,
            },
        )
        .await
    }

    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetRole { id, tx })
            .await
    }

    async fn delete_role(&self, id: Snowflake) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::DeleteRole { id, tx })
            .await
    }

    async fn store_emoji(&self, emoji: Emoji, guild_id: Snowflake) -> Result<()> {
        self.store_emojis(vec![emoji], guild_id).await
    }

    async fn store_emojis(&self, emojis: Vec<Emoji>, guild_id: Snowflake) -> Result<()> {
        if !self.opts.emojis {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(
            rx,
            CachePayload::StoreEmojis {
                emojis,
                guild_id,
                tx,
            },
        )
        .await
    }

    async fn get_emoji(&self, id: Snowflake) -> Result<Option<Emoji>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::GetEmoji { id, tx })
            .await
    }

    async fn delete_emoji(&self, id: Snowflake) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::DeleteEmoji { id, tx })
            .await
    }

    async fn store_voice_state(&self, voice_state: VoiceState) -> Result<()> {
        self.store_voice_states(vec![voice_state]).await
    }

    async fn store_voice_states(&self, voice_states: Vec<VoiceState>) -> Result<()> {
        if !self.opts.voice_states {
            return Ok(());
        }

        let (tx, rx) = oneshot::channel();
        self.send_payload(rx, CachePayload::StoreVoiceState { voice_states, tx })
            .await
    }

    async fn get_voice_state(
        &self,
        user_id: Snowflake,
        guild_id: Snowflake,
    ) -> Result<Option<VoiceState>> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(
            rx,
            CachePayload::GetVoiceState {
                user_id,
                guild_id,
                tx,
            },
        )
        .await
    }

    async fn delete_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.send_payload(
            rx,
            CachePayload::DeleteVoiceState {
                user_id,
                guild_id,
                tx,
            },
        )
        .await
    }
}
