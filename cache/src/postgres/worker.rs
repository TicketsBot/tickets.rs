use std::sync::Arc;

use crate::{CacheError, Options, Result};
use deadpool_postgres::Object;
use model::channel::Channel;
use model::guild::{Emoji, Guild, Member, Role};
use model::user::User;
use model::Snowflake;
use tokio::task::JoinSet;

#[derive(Debug, Clone)]
pub struct Worker {
    options: Options,
    conn: Arc<Conn>,
}

type Conn = Object;

impl Worker {
    pub fn new(options: Options, conn: Conn) -> Worker {
        Worker {
            options,
            conn: Arc::new(conn),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn create_schema(&self) -> Result<()> {
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
            r#"CREATE INDEX IF NOT EXISTS channels_guild_id ON channels("guild_id");"#,
            r#"CREATE INDEX IF NOT EXISTS members_guild_id ON members("guild_id");"#,
            r#"CREATE INDEX IF NOT EXISTS member_user_id ON members("user_id");"#,
            r#"CREATE INDEX IF NOT EXISTS roles_guild_id ON roles("guild_id");"#,
            r#"CREATE INDEX IF NOT EXISTS emojis_guild_id ON emojis("guild_id");"#,
            r#"CREATE INDEX IF NOT EXISTS voice_states_guild_id ON voice_states("guild_id");"#,
            r#"CREATE INDEX IF NOT EXISTS voice_states_user_id ON voice_states("user_id");"#,
        ];

        for query in queries {
            self.conn
                .execute(query, &[])
                .await
                .map_err(CacheError::DatabaseError)?;
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, guilds))]
    pub async fn store_guilds(&self, guilds: Vec<Guild>) -> Result<()> {
        if guilds.is_empty() {
            return Ok(());
        }

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

        let mut join_set = JoinSet::new();

        let cloned = Arc::clone(&self.conn);
        join_set.spawn(async move {
            cloned.simple_query(query.as_str()).await.map_err(CacheError::DatabaseError).map(|_| ())
        });

        for guild in guilds {
            if self.options.channels {
                if let Some(channels) = guild.channels {
                    let cloned = self.clone();
                    let guild_id = guild.id;
                    join_set.spawn(async move {
                        cloned.store_channels(channels, guild_id).await
                    });
                }
            }

            if self.options.threads {
                if let Some(threads) = guild.threads {
                    let cloned = self.clone();
                    let guild_id = guild.id;
                    join_set.spawn(async move {
                        cloned.store_channels(threads, guild_id).await
                    });
                }
            }

            if self.options.roles {
                let cloned = self.clone();
                let roles = guild.roles;
                let id = guild.id;
                join_set.spawn(async move {
                    cloned.store_roles(roles, id).await
                });
            }

            if self.options.emojis {
                let cloned = self.clone();
                let emojis = guild.emojis;
                let id = guild.id;
                join_set.spawn(async move {
                    cloned.store_emojis(emojis, id).await
                });
            }
        }

        while let Some(res) = join_set.join_next().await {
            res??;
        }

        Ok(())
    }

    pub async fn get_guild(&self, id: Snowflake) -> Result<Option<Guild>> {
        /*self.conn
            .query(
                r#"SELECT "data" FROM guilds WHERE "guild_id" = $1;"#,
                &[&(id.0 as i64)],
            )
            .await;
        Ok(None)*/

        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_guild(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM guilds WHERE "guild_id" = $1;"#;
        self.conn
            .execute(query, &[&(id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_guild_count(&self) -> Result<usize> {
        let query = r#"SELECT COUNT(guild_id) FROM guilds;"#;

        let row = self.conn
            .query_one(query, &[])
            .await
            .map_err(CacheError::DatabaseError)?;

        let count: i64 = row.try_get(0).map_err(CacheError::DatabaseError)?;
        Ok(count as usize)
    }

    #[tracing::instrument(skip(self, channels), fields(channel_count = channels.len()))]
    pub async fn store_channels(&self, channels: Vec<Channel>, guild_id: Snowflake) -> Result<()> {
        if channels.is_empty() {
            return Ok(());
        }

        let mut query =
            String::from(r#"INSERT INTO channels("channel_id", "guild_id", "data") VALUES"#);

        let mut first = true;
        channels
            .into_iter()
            .filter(|c| self.options.threads || !c.channel_type.is_thread())
            .try_for_each(|c| -> Result<()> {
                if first {
                    first = false;
                } else {
                    query.push(',');
                }
                
                let encoded = serde_json::to_string(&c).map_err(CacheError::JsonError)?;
                query.push_str(&format!(
                    r#"({}, {}, {}::jsonb)"#,
                    c.id.0,
                    guild_id.0,
                    quote_literal(encoded)
                ));

                Ok(())
            })?;

        if first {
            return Ok(());
        }

        query.push_str(r#" ON CONFLICT("channel_id") DO UPDATE SET "data" = excluded.data;"#);

        self.conn
            .simple_query(&query[..])
            .await
            .map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    pub async fn get_channel(&self, id: Snowflake) -> Result<Option<Channel>> {
        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_channel(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM channels WHERE "channel_id" = $1;"#;
        self.conn
            .execute(query, &[&(id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, users), fields(user_count = users.len()))]
    pub async fn store_users(&self, users: Vec<User>) -> Result<()> {
        if users.is_empty() {
            return Ok(());
        }

        let mut query = String::from(r#"INSERT INTO users("user_id", "data", "last_seen") VALUES"#);

        let mut first = true;
        for user in users {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&user).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}::jsonb, NOW())"#,
                user.id.0,
                quote_literal(encoded)
            ));
        }

        query.push_str(r#" ON CONFLICT("user_id") DO UPDATE SET "data" = excluded.data, "last_seen" = excluded.last_seen;"#);

        self.conn
            .simple_query(&query[..])
            .await
            .map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    pub async fn get_user(&self, id: Snowflake) -> Result<Option<User>> {
        /*let row = self
            .client
            .query_one(
                r#"SELECT "data" FROM users WHERE "user_id" = $1;"#,
                &[&(id.0 as i64)],
            )
            .await
            .map_err(CacheError::DatabaseError)?;
        let data: &str = row.get(0);

        Ok(None)*/

        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_user(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM users WHERE "user_id" = $1;"#;
        self.conn
            .execute(query, &[&(id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, members), fields(member_count = members.len()))]
    pub async fn store_members(&self, members: Vec<Member>, guild_id: Snowflake) -> Result<()> {
        if members.is_empty() {
            return Ok(());
        }

        let mut query = String::from(
            r#"INSERT INTO members("guild_id", "user_id", "data", "last_seen") VALUES"#,
        );

        let mut first = true;
        for member in members {
            let user = match &member.user {
                Some(user) => user,
                None => continue,
            };

            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&member).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}, {}::jsonb, NOW())"#,
                guild_id,
                user.id,
                quote_literal(encoded)
            ));
        }

        query.push_str(
            r#" ON CONFLICT("guild_id", "user_id") DO UPDATE SET "data" = excluded.data, "last_seen" = excluded.last_seen;"#,
        );

        self.conn
            .simple_query(&query[..])
            .await
            .map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    pub async fn get_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<Member>> {
        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM members WHERE "guild_id" = $1 AND "user_id" = $2;"#;
        self.conn
            .execute(query, &[&(guild_id.0 as i64), &(user_id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, roles), fields(role_count = roles.len()))]
    pub async fn store_roles(&self, roles: Vec<Role>, guild_id: Snowflake) -> Result<()> {
        if roles.is_empty() {
            return Ok(());
        }

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

        self.conn
            .simple_query(&query[..])
            .await
            .map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    pub async fn get_role(&self, id: Snowflake) -> Result<Option<Role>> {
        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_role(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM roles WHERE "role_id" = $1;"#;
        self.conn
            .execute(query, &[&(id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, emojis), fields(emoji_count = emojis.len()))]
    pub async fn store_emojis(&self, emojis: Vec<Emoji>, guild_id: Snowflake) -> Result<()> {
        if emojis.is_empty() {
            return Ok(());
        }

        let mut query =
            String::from(r#"INSERT INTO emojis("emoji_id", "guild_id", "data") VALUES"#);

        let mut first = true;
        for emoji in emojis {
            if emoji.id.is_none() {
                continue;
            }

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

        if first {
            return Ok(());
        }

        query.push_str(
            r#" ON CONFLICT("emoji_id", "guild_id") DO UPDATE SET "data" = excluded.data;"#,
        );

        self.conn
            .simple_query(&query[..])
            .await
            .map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    pub async fn get_emoji(&self, emoji_id: Snowflake) -> Result<Option<Emoji>> {
        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_emoji(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM emojis WHERE "emoji_id" = $1;"#;
        self.conn
            .execute(query, &[&(id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
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
