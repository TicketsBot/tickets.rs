use crate::{Cache, CacheError, Options, Result};
use model::user::User;
use model::Snowflake;

use async_trait::async_trait;

use deadpool_postgres::{tokio_postgres, Object};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use model::channel::Channel;
use model::guild::{Emoji, Guild, Member, Role, VoiceState};
use tracing::info;
use std::cmp::Ordering::Equal;

#[cfg(feature = "metrics")]
use lazy_static::lazy_static;

#[cfg(feature = "metrics")]
use std::time::Instant;

#[cfg(feature = "metrics")]
use prometheus::{HistogramOpts, HistogramVec};

#[cfg(feature = "metrics")]
lazy_static! {
    static ref HISTOGRAM: HistogramVec = HistogramVec::new(
        HistogramOpts::new("cache_timings", "Cache Timings"),
        &["table"],
    )
    .expect("Failed to construct histogram");
}

pub struct PostgresCache {
    opts: Options,
    //tx: mpsc::UnboundedSender<CachePayload>,
    pool: Pool,
}

impl PostgresCache {
    /// panics if URI is invalid
    pub async fn connect(uri: String, opts: Options, workers: usize) -> Result<PostgresCache> {
        info!("Building Postgres cache");
        let cfg = uri.parse::<tokio_postgres::Config>()?;

        let manager_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };

        let manager = Manager::from_config(cfg, tokio_postgres::NoTls, manager_config);
        let pool = Pool::builder(manager).max_size(workers).build()?;

        info!("Build Postgres cache with {workers} connections");

        Ok(PostgresCache { opts, pool })
    }

    pub async fn create_schema(&self) -> Result<()> {
        let mut batch = String::new();

        vec![
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
        ].iter().for_each(|query| {
            batch.push_str(query);
        });

        self.get_conn().await?.batch_execute(&batch[..]).await?;

        Ok(())
    }

    async fn get_conn(&self) -> Result<Object> {
        Ok(self.pool.get().await?)
    }
}

#[async_trait]
impl Cache for PostgresCache {
    async fn store_guild(&self, guild: Guild) -> Result<()> {
        self.store_guilds(vec![guild]).await
    }

    async fn store_guilds(&self, mut guilds: Vec<Guild>) -> Result<()> {
        if guilds.is_empty() {
            return Ok(());
        }

        #[cfg(feature = "metrics")]
        let start = Instant::now();

        guilds.sort_by(|g1, g2| g1.id.cmp(&g2.id));
        guilds.dedup();

        let mut query = String::from(r#"INSERT INTO guilds("guild_id", "data") VALUES"#);

        let mut first = true;
        for guild in guilds.iter() {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(guild).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}::jsonb)"#,
                guild.id.0,
                quote_literal(encoded)
            ));
        }

        query.push_str(r#" ON CONFLICT("guild_id") DO UPDATE SET "data" = excluded.data;"#);

        self.get_conn().await?
            .simple_query(&query[..])
            .await?;

        #[cfg(feature = "metrics")]
        HISTOGRAM
            .with_label_values(&["guilds"])
            .observe((Instant::now() - start).as_secs_f64());

        // cache objects on guild
        let mut res: Result<()> = Ok(());

        // TODO: check opts
        for guild in guilds {
            if let Some(channels) = guild.channels {
                if let Err(e) = self.store_channels(channels).await {
                    res = Err(e);
                }
            }
            if let Some(threads) = guild.threads {
                if let Err(e) = self.store_channels(threads).await {
                    res = Err(e);
                }
            }

            if let Some(members) = guild.members {
                let users = members.iter().filter_map(|m| m.user.clone()).collect();

                if let Err(e) = self.store_members(members, guild.id).await {
                    res = Err(e);
                }

                if let Err(e) = self.store_users(users).await {
                    res = Err(e)
                }
            }

            if let Err(e) = self.store_roles(guild.roles, guild.id).await {
                res = Err(e);
            }

            if self.opts.emojis {
                if let Err(e) = self.store_emojis(guild.emojis, guild.id).await {
                    res = Err(e)
                }    
            }

            if self.opts.voice_states {
                if let Some(voice_states) = guild.voice_states {
                    if let Err(e) = self.store_voice_states(voice_states).await {
                        res = Err(e)
                    }
                }
            }
        }

        res
    }

    async fn get_guild(&self, id: Snowflake) -> Result<Option<Guild>> {
        unimplemented!()
    }

    async fn delete_guild(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM guilds WHERE "guild_id" = $1;"#;
        self.get_conn()
            .await?
            .execute(query, &[&(id.0 as i64)])
            .await?;

        Ok(())
    }

    async fn get_guild_count(&self) -> Result<usize> {
        let query = r#"SELECT COUNT(guild_id) FROM guilds;"#;

        let row = self.get_conn().await?.query_one(query, &[]).await?;

        let count: i64 = row.try_get(0).map_err(CacheError::DatabaseError)?;
        Ok(count as usize)
    }

    async fn store_channel(&self, channel: Channel) -> Result<()> {
        self.store_channels(vec![channel]).await
    }

    async fn store_channels(&self, channels: Vec<Channel>) -> Result<()> {
        if !self.opts.channels {
            return Ok(());
        }

        if channels.is_empty() {
            return Ok(());
        }

        let mut channels = channels
            .into_iter()
            .filter(|c| c.guild_id.is_some())
            .collect::<Vec<Channel>>();

        channels.sort_by(|c1, c2| c1.id.cmp(&c2.id));
        channels.dedup();

        let mut query =
            String::from(r#"INSERT INTO channels("channel_id", "guild_id", "data") VALUES"#);

        let mut first = true;
        for channel in channels {
            // TODO: Cache DMs?
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&channel)?;
            query.push_str(&format!(
                r#"({}, {}, {}::jsonb)"#,
                channel.id.0,
                channel.guild_id.unwrap().0,
                quote_literal(encoded)
            ));
        }

        query.push_str(
            r#" ON CONFLICT("channel_id", "guild_id") DO UPDATE SET "data" = excluded.data;"#,
        );

        self.get_conn().await?
            .simple_query(&query[..])
            .await?;

        Ok(())
    }

    async fn get_channel(&self, id: Snowflake) -> Result<Option<Channel>> {
        unimplemented!()
    }

    async fn delete_channel(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM channels WHERE "channel_id" = $1;"#;
        self.get_conn().await?
            .execute(query, &[&(id.0 as i64)])
            .await?;

        Ok(())
    }

    async fn store_user(&self, user: User) -> Result<()> {
        self.store_users(vec![user]).await
    }

    async fn store_users(&self, mut users: Vec<User>) -> Result<()> {
        if !self.opts.users {
            return Ok(());
        }

        if users.is_empty() {
            return Ok(());
        }

        users.sort_by(|one, two| one.id.cmp(&two.id));
        users.dedup();

        let mut query = String::from(r#"INSERT INTO users("user_id", "data") VALUES"#);

        let mut first = true;
        for user in users {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&user).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}::jsonb)"#,
                user.id.0,
                quote_literal(encoded)
            ));
        }

        query.push_str(r#" ON CONFLICT("user_id") DO UPDATE SET "data" = excluded.data;"#);

        self.get_conn().await?
            .simple_query(&query[..])
            .await?;

        Ok(())
    }

    async fn get_user(&self, id: Snowflake) -> Result<Option<User>> {
        unimplemented!()
    }

    async fn delete_user(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM users WHERE "user_id" = $1;"#;
        self.get_conn().await?
            .execute(query, &[&(id.0 as i64)])
            .await?;

        Ok(())
    }

    async fn store_member(&self, member: Member, guild_id: Snowflake) -> Result<()> {
        self.store_members(vec![member], guild_id).await
    }

    async fn store_members(&self, members: Vec<Member>, guild_id: Snowflake) -> Result<()> {
        if !self.opts.members {
            return Ok(());
        }

        let mut members = members
            .into_iter()
            .filter(|m| m.user.is_some())
            .collect::<Vec<Member>>();

        if members.is_empty() {
            return Ok(());
        }

        members.sort_by(|one, two| {
            if let (Some(user_one), Some(user_two)) = (&one.user, &two.user) {
                return user_one.id.cmp(&user_two.id);
            }

            Equal // idk
        });

        members.dedup_by(|one, two| {
            if let (Some(user_one), Some(user_two)) = (&one.user, &two.user) {
                return user_one.id == user_two.id;
            }

            false
        });

        let mut query =
            String::from(r#"INSERT INTO members("guild_id", "user_id", "data") VALUES"#);

        let mut first = true;
        for member in members {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&member).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}, {}::jsonb)"#,
                guild_id,
                member.user.as_ref().unwrap().id,
                quote_literal(encoded)
            ));
        }

        query.push_str(
            r#" ON CONFLICT("guild_id", "user_id") DO UPDATE SET "data" = excluded.data;"#,
        );

        self.get_conn().await?
            .simple_query(&query[..])
            .await?;

        Ok(())
    }

    async fn get_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<Member>> {
        unimplemented!()
    }

    async fn delete_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM members WHERE "guild_id" = $1 AND "user_id" = $2;"#;
        self.get_conn().await?
            .execute(query, &[&(guild_id.0 as i64), &(user_id.0 as i64)])
            .await?;

        Ok(())
    }

    async fn store_role(&self, role: Role, guild_id: Snowflake) -> Result<()> {
        self.store_roles(vec![role], guild_id).await
    }

    async fn store_roles(&self, mut roles: Vec<Role>, guild_id: Snowflake) -> Result<()> {
        if !self.opts.roles {
            return Ok(());
        }

        if roles.is_empty() {
            return Ok(());
        }

        roles.sort_by(|r1, r2| r1.id.cmp(&r2.id));
        roles.dedup();

        let mut query = String::from(r#"INSERT INTO roles("role_id", "guild_id", "data") VALUES"#);

        let mut first = true;
        for role in roles {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&role).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}, {}::jsonb)"#,
                role.id.0,
                guild_id.0,
                quote_literal(encoded)
            ));
        }

        query.push_str(
            r#" ON CONFLICT("role_id", "guild_id") DO UPDATE SET "data" = excluded.data;"#,
        );

        self.get_conn().await?
            .simple_query(&query[..])
            .await?;

        Ok(())
    }

    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>> {
        unimplemented!()
    }

    async fn delete_role(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM roles WHERE "role_id" = $1;"#;
        self.get_conn().await?
            .execute(query, &[&(id.0 as i64)])
            .await?;

        Ok(())
    }

    async fn store_emoji(&self, emoji: Emoji, guild_id: Snowflake) -> Result<()> {
        self.store_emojis(vec![emoji], guild_id).await
    }

    async fn store_emojis(&self, emojis: Vec<Emoji>, guild_id: Snowflake) -> Result<()> {
        if !self.opts.emojis {
            return Ok(());
        }

        if emojis.is_empty() {
            return Ok(());
        }

        let mut emojis = emojis
            .into_iter()
            .filter(|e| e.id.is_some())
            .collect::<Vec<Emoji>>();

        emojis.sort_by(|e1, e2| e1.id.cmp(&e2.id));
        emojis.dedup();

        let mut query =
            String::from(r#"INSERT INTO emojis("emoji_id", "guild_id", "data") VALUES"#);

        let mut first = true;
        for emoji in emojis {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&emoji).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}, {}::jsonb)"#,
                emoji.id.unwrap(),
                guild_id,
                quote_literal(encoded)
            ));
        }

        query.push_str(
            r#" ON CONFLICT("emoji_id", "guild_id") DO UPDATE SET "data" = excluded.data;"#,
        );

        self.get_conn().await?
            .simple_query(&query[..])
            .await?;

        Ok(())
    }

    async fn get_emoji(&self, id: Snowflake) -> Result<Option<Emoji>> {
        unimplemented!()
    }

    async fn delete_emoji(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM emojis WHERE "emoji_id" = $1;"#;
        self.get_conn().await?
            .execute(query, &[&(id.0 as i64)])
            .await?;

        Ok(())
    }

    async fn store_voice_state(&self, voice_state: VoiceState) -> Result<()> {
        self.store_voice_states(vec![voice_state]).await
    }

    async fn store_voice_states(&self, voice_states: Vec<VoiceState>) -> Result<()> {
        if !self.opts.voice_states {
            return Ok(());
        }

        if voice_states.is_empty() {
            return Ok(());
        }

        let mut voice_states = voice_states
            .into_iter()
            .filter(|vs| vs.guild_id.is_some())
            .collect::<Vec<VoiceState>>();

        // TODO: Sort
        voice_states.dedup();

        let mut query =
            String::from(r#"INSERT INTO voice_states("guild_id", "user_id", "data") VALUES"#);

        let mut first = true;
        for voice_state in voice_states {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&voice_state).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}, {}::jsonb)"#,
                voice_state.guild_id.unwrap(),
                voice_state.user_id,
                quote_literal(encoded)
            ));
        }

        query.push_str(
            r#" ON CONFLICT("guild_id", "user_id") DO UPDATE SET "data" = excluded.data;"#,
        );

        self.get_conn().await?
            .simple_query(&query[..])
            .await?;

        Ok(())
    }

    async fn get_voice_state(
        &self,
        user_id: Snowflake,
        guild_id: Snowflake,
    ) -> Result<Option<VoiceState>> {
        unimplemented!()
    }

    async fn delete_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM voice_states WHERE "guild_id" = $1 AND "user_id" = $2;"#;
        self.get_conn().await?
            .execute(query, &[&(guild_id.0 as i64), &(user_id.0 as i64)])
            .await?;

        Ok(())
    }
}

fn quote_literal(s: String) -> String {
    let s = s.replace("'", "''");

    if s.contains(r#"\"#) {
        let s = s.replace(r#"\"#, r#"\\"#);
        format!(" E'{}'", s)
    } else {
        format!("'{}'", s)
    }
}
