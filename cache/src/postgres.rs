use super::{Cache, Options};
use model::user::User;
use model::Snowflake;
use crate::CacheError;

use async_trait::async_trait;

use sqlx::postgres::{PgPool, PgPoolOptions};
use model::channel::Channel;
use model::guild::{Role, Guild, Member, Emoji, VoiceState};
use serde_json::Value;
use futures_util::core_reexport::cmp::Ordering::Equal;

pub struct PostgresCache {
    opts: Options,
    pool: PgPool,
}

impl PostgresCache {
    /// panics if URI is invalid
    pub async fn connect(uri: &str, opts: Options, pg_opts: PgPoolOptions) -> Result<PostgresCache, CacheError> {
        let pool = pg_opts.connect(uri).await?;

        Ok(PostgresCache {
            opts,
            pool,
        })
    }

    pub async fn create_schema(&self) -> Result<(), CacheError> {
        // create tables
        sqlx::query(r#"CREATE TABLE IF NOT EXISTS guilds("guild_id" int8 NOT NULL UNIQUE, "data" jsonb NOT NULL, PRIMARY KEY("guild_id"));"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        sqlx::query(r#"CREATE TABLE IF NOT EXISTS channels("channel_id" int8 NOT NULL UNIQUE, "guild_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("channel_id", "guild_id"));"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        sqlx::query(r#"CREATE TABLE IF NOT EXISTS users("user_id" int8 NOT NULL UNIQUE, "data" jsonb NOT NULL, PRIMARY KEY("user_id"));"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        sqlx::query(r#"CREATE TABLE IF NOT EXISTS members("guild_id" int8 NOT NULL, "user_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("guild_id", "user_id"));"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        sqlx::query(r#"CREATE TABLE IF NOT EXISTS roles("role_id" int8 NOT NULL UNIQUE, "guild_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("role_id", "guild_id"));"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        sqlx::query(r#"CREATE TABLE IF NOT EXISTS emojis("emoji_id" int8 NOT NULL UNIQUE, "guild_id" int8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("emoji_id", "guild_id"));"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        sqlx::query(r#"CREATE TABLE IF NOT EXISTS voice_states("guild_id" int8 NOT NULL, "user_id" INT8 NOT NULL, "data" jsonb NOT NULL, PRIMARY KEY("guild_id", "user_id"));"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;

        // create indexes
        sqlx::query(r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS channels_guild_id ON channels("guild_id");"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        sqlx::query(r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS members_guild_id ON members("guild_id");"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        sqlx::query(r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS member_user_id ON members("user_id");"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        sqlx::query(r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS roles_guild_id ON roles("guild_id");"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        sqlx::query(r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS emojis_guild_id ON emojis("guild_id");"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        sqlx::query(r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS voice_states_guild_id ON voice_states("guild_id");"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        sqlx::query(r#"CREATE INDEX CONCURRENTLY IF NOT EXISTS voice_states_user_id ON voice_states("user_id");"#).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;

        Ok(())
    }
}

#[async_trait]
impl Cache for PostgresCache {
    async fn store_guild(&self, guild: &Guild) -> Result<(), CacheError> {
        self.store_guilds(vec![guild]).await
    }

    async fn store_guilds(&self, mut guilds: Vec<&Guild>) -> Result<(), CacheError> {
        if !self.opts.guilds {
            return Ok(());
        }

        if guilds.len() == 0 {
            return Ok(());
        }

        guilds.sort_by(|g1, g2| g1.id.cmp(&g2.id));
        guilds.dedup();

        let mut query = String::from(r#"INSERT INTO guilds("guild_id", "data") VALUES"#);

        let mut first = true;
        for guild in guilds.iter() {
            if first {
                first = false;
            } else {
                query.push_str(",");
            }

            let encoded = serde_json::to_string(guild).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}::jsonb)"#, guild.id.0, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("guild_id") DO UPDATE SET "data" = excluded.data;"#);

        // TODO: Don't prepare statement
        sqlx::query(&query).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;

        // cache objects on guild
        let mut res: Result<(), CacheError> = Ok(());

        // TODO: check opts before we convert to borrows
        for guild in guilds {
            if let Some(channels) = &guild.channels {
                if let Err(e) = self.store_channels(channels.iter().collect()).await {
                    res = Err(e);
                }
            }

            if let Some(members) = &guild.members {
                if let Err(e) = self.store_members(members.iter().collect(), guild.id).await {
                    res = Err(e);
                }

                let users = members
                    .iter()
                    .map(|m| m.user.as_ref())
                    .filter(|m| m.is_some())
                    .map(|m| m.unwrap())
                    .collect();

                if let Err(e) = self.store_users(users).await {
                    res = Err(e)
                }
            }

            if let Err(e) = self.store_roles(guild.roles.iter().collect(), guild.id).await {
                res = Err(e);
            }

            if let Err(e) = self.store_emojis(guild.emojis.iter().collect(), guild.id).await {
                res = Err(e)
            }

            if let Some(voice_states) = &guild.voice_states {
                if let Err(e) = self.store_voice_states(voice_states.iter().collect()).await {
                    res = Err(e)
                }
            }
        }

        res
    }

    async fn get_guild(&self, id: Snowflake) -> Result<Option<Guild>, CacheError> {
        match sqlx::query!(r#"SELECT "data" FROM guilds WHERE "guild_id" = $1;"#, id.0 as i64).fetch_one(&self.pool).await {
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(CacheError::DatabaseError(e)),
            Ok(r) => {
                let r: Value = r.data;
                match r {
                    Value::Object(mut m) => {
                        m.insert("id".to_owned(), Value::Number(serde_json::Number::from(id.0)));
                        Ok(Some(serde_json::from_value::<Guild>(Value::Object(m))?))
                    }
                    _ => Err(CacheError::WrongType()),
                }
            },
        }
    }

    async fn delete_guild(&self, id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM guilds WHERE "guild_id" = $1;"#;
        sqlx::query(query).bind(id.0 as i64).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    async fn store_channel(&self, channel: &Channel) -> Result<(), CacheError> {
        self.store_channels(vec![channel]).await
    }

    async fn store_channels(&self, channels: Vec<&Channel>) -> Result<(), CacheError> {
        if !self.opts.channels {
            return Ok(());
        }

        let mut channels = channels.into_iter().filter(|c| c.guild_id.is_some()).collect::<Vec<&Channel>>();

        if channels.len() == 0 {
            return Ok(());
        }

        channels.sort_by(|c1, c2| c1.id.cmp(&c2.id));
        channels.dedup();

        let mut query = String::from(r#"INSERT INTO channels("channel_id", "guild_id", "data") VALUES"#);

        let mut first = true;
        for channel in channels { // TODO: Cache DMs?
            if first {
                first = false;
            } else {
                query.push_str(",");
            }

            let encoded = serde_json::to_string(channel).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}, {}::jsonb)"#, channel.id.0, channel.guild_id.unwrap().0, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("channel_id", "guild_id") DO UPDATE SET "data" = excluded.data;"#);

        // TODO: Don't prepare statement
        sqlx::query(&query).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_channel(&self, id: Snowflake) -> Result<Option<Channel>, CacheError> {
        match sqlx::query!(r#"SELECT "guild_id", "data" FROM channels WHERE "channel_id" = $1;"#, id.0 as i64).fetch_one(&self.pool).await {
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(CacheError::DatabaseError(e)),
            Ok(r) => {
                let data: Value = r.data;
                match data {
                    Value::Object(mut m) => {
                        m.insert("channel_id".to_owned(), Value::Number(serde_json::Number::from(id.0)));
                        m.insert("guild_id".to_owned(), Value::Number(serde_json::Number::from(r.guild_id)));
                        Ok(Some(serde_json::from_value::<Channel>(Value::Object(m))?))
                    }
                    _ => Err(CacheError::WrongType()),
                }
            },
        }
    }

    async fn delete_channel(&self, id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM channels WHERE "channel_id" = $1;"#;
        sqlx::query(query).bind(id.0 as i64).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    async fn store_user(&self, user: &User) -> Result<(), CacheError> {
        self.store_users(vec![user]).await
    }

    async fn store_users(&self, mut users: Vec<&User>) -> Result<(), CacheError> {
        if !self.opts.users {
            return Ok(());
        }

        if users.len() == 0 {
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
                query.push_str(",");
            }

            let encoded = serde_json::to_string(user).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}::jsonb)"#, user.id.0, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("user_id") DO UPDATE SET "data" = excluded.data;"#);

        // TODO: Don't prepare statement
        if let Err(e) = sqlx::query(&query).execute(&self.pool).await.map_err(CacheError::DatabaseError) {
            return e.into();
        }

        Ok(())
    }

    async fn get_user(&self, id: Snowflake) -> Result<Option<User>, CacheError> {
        match sqlx::query!(r#"SELECT "data" FROM users WHERE "user_id" = $1;"#, id.0 as i64).fetch_one(&self.pool).await {
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(CacheError::DatabaseError(e)),
            Ok(r) => {
                let data: Value = r.data;
                match data {
                    Value::Object(mut m) => {
                        m.insert("user_id".to_owned(), Value::Number(serde_json::Number::from(id.0)));
                        Ok(Some(serde_json::from_value::<User>(Value::Object(m))?))
                    }
                    _ => Err(CacheError::WrongType()),
                }
            },
        }
    }

    async fn delete_user(&self, id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM users WHERE "user_id" = $1;"#;
        sqlx::query(query).bind(id.0 as i64).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    async fn store_member(&self, member: &Member, guild_id: Snowflake) -> Result<(), CacheError> {
        self.store_members(vec![member], guild_id).await
    }

    async fn store_members(&self, members: Vec<&Member>, guild_id: Snowflake) -> Result<(), CacheError> {
        if !self.opts.members {
            return Ok(());
        }

        let mut members = members.into_iter().filter(|m| m.user.is_some()).collect::<Vec<&Member>>();

        if members.len() == 0 {
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

        let mut query = String::from(r#"INSERT INTO members("guild_id", "user_id", "data") VALUES"#);

        let mut first = true;
        for member in members {
            if first {
                first = false;
            } else {
                query.push_str(",");
            }

            let encoded = serde_json::to_string(member).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}, {}::jsonb)"#, guild_id, member.user.as_ref().unwrap().id, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("guild_id", "user_id") DO UPDATE SET "data" = excluded.data;"#);

        // TODO: Don't prepare statement
        sqlx::query(&query).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<Member>, CacheError> {
        match sqlx::query!(r#"SELECT "data" FROM members WHERE "guild_id" = $1 AND "user_id" = $2;"#, user_id.0 as i64, guild_id.0 as i64).fetch_one(&self.pool).await {
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(CacheError::DatabaseError(e)),
            Ok(r) => {
                let data: Value = r.data;
                match data {
                    Value::Object(mut m) => {
                        m.insert("user_id".to_owned(), Value::Number(serde_json::Number::from(user_id.0)));
                        m.insert("guild_id".to_owned(), Value::Number(serde_json::Number::from(guild_id.0)));
                        Ok(Some(serde_json::from_value::<Member>(Value::Object(m))?))
                    }
                    _ => Err(CacheError::WrongType()),
                }
            },
        }
    }

    async fn delete_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM members WHERE "guild_id" = $1 AND "user_id" = $2;"#;
        sqlx::query(query).bind(guild_id.0 as i64).bind(user_id.0 as i64).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    async fn store_role(&self, role: &Role, guild_id: Snowflake) -> Result<(), CacheError> {
        self.store_roles(vec![role], guild_id).await
    }

    async fn store_roles(&self, mut roles: Vec<&Role>, guild_id: Snowflake) -> Result<(), CacheError> {
        if !self.opts.roles {
            return Ok(());
        }

        if roles.len() == 0 {
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
                query.push_str(",");
            }

            let encoded = serde_json::to_string(role).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}, {}::jsonb)"#, role.id.0 as i64, guild_id.0 as i64, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("role_id", "guild_id") DO UPDATE SET "data" = excluded.data;"#);

        // TODO: Don't prepare statement
        sqlx::query(&query).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>, CacheError> {
        match sqlx::query!(r#"SELECT "data" FROM roles WHERE "role_id" = $1;"#, id.0 as i64).fetch_one(&self.pool).await {
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(CacheError::DatabaseError(e)),
            Ok(r) => {
                let data: Value = r.data;
                match data {
                    Value::Object(mut m) => {
                        m.insert("id".to_owned(), Value::Number(serde_json::Number::from(id.0)));
                        Ok(Some(serde_json::from_value::<Role>(Value::Object(m))?))
                    }
                    _ => Err(CacheError::WrongType()),
                }
            },
        }
    }

    async fn delete_role(&self, id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM roles WHERE "role_id" = $1;"#;
        sqlx::query(query).bind(id.0 as i64).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    async fn store_emoji(&self, emoji: &Emoji, guild_id: Snowflake) -> Result<(), CacheError> {
        self.store_emojis(vec![emoji], guild_id).await
    }

    async fn store_emojis(&self, emojis: Vec<&Emoji>, guild_id: Snowflake) -> Result<(), CacheError> {
        if !self.opts.emojis {
            return Ok(());
        }

        let mut emojis = emojis.into_iter().filter(|e| e.id.is_some()).collect::<Vec<&Emoji>>();

        if emojis.len() == 0 {
            return Ok(());
        }

        emojis.sort_by(|e1, e2| e1.id.cmp(&e2.id));
        emojis.dedup();

        let mut query = String::from(r#"INSERT INTO emojis("emoji_id", "guild_id", "data") VALUES"#);

        let mut first = true;
        for emoji in emojis {
            if first {
                first = false;
            } else {
                query.push_str(",");
            }

            let encoded = serde_json::to_string(emoji).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}, {}::jsonb)"#, emoji.id.unwrap(), guild_id, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("emoji_id", "guild_id") DO UPDATE SET "data" = excluded.data;"#);

        // TODO: Don't prepare statement
        // sqlx::query(&query).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_emoji(&self, emoji_id: Snowflake) -> Result<Option<Emoji>, CacheError> {
        match sqlx::query!(r#"SELECT "data" FROM emojis WHERE "emoji_id" = $1;"#, emoji_id.0 as i64).fetch_one(&self.pool).await {
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(CacheError::DatabaseError(e)),
            Ok(r) => {
                let data: Value = r.data;
                match data {
                    Value::Object(mut m) => {
                        m.insert("id".to_owned(), Value::Number(serde_json::Number::from(emoji_id.0)));
                        Ok(Some(serde_json::from_value::<Emoji>(Value::Object(m))?))
                    }
                    _ => Err(CacheError::WrongType()),
                }
            },
        }
    }

    async fn delete_emoji(&self, emoji_id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM emojis WHERE "emoji_id" = $1;"#;
        sqlx::query(query).bind(emoji_id.0 as i64).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    async fn store_voice_state(&self, voice_state: &VoiceState) -> Result<(), CacheError> {
        self.store_voice_states(vec![voice_state]).await
    }

    async fn store_voice_states(&self, voice_states: Vec<&VoiceState>) -> Result<(), CacheError> {
        if !self.opts.voice_states {
            return Ok(());
        }

        let mut voice_states = voice_states.into_iter().filter(|vs| vs.guild_id.is_some()).collect::<Vec<&VoiceState>>();

        if voice_states.len() == 0 {
            return Ok(());
        }

        // TODO: Sort
        voice_states.dedup();

        let mut query = String::from(r#"INSERT INTO voice_states("guild_id", "user_id", "data") VALUES"#);

        let mut first = true;
        for voice_state in voice_states {
            if first {
                first = false;
            } else {
                query.push_str(",");
            }

            let encoded = serde_json::to_string(voice_state).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}, {}::jsonb)"#, voice_state.guild_id.unwrap(), voice_state.user_id, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("guild_id", "user_id") DO UPDATE SET "data" = excluded.data;"#);

        // TODO: Don't prepare statement
        sqlx::query(&query).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<VoiceState>, CacheError> {
        match sqlx::query!(r#"SELECT "data" FROM voice_states WHERE "guild_id" = $1 AND "user_id" = $2;"#, guild_id.0 as i64, user_id.0 as i64).fetch_one(&self.pool).await {
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(CacheError::DatabaseError(e)),
            Ok(r) => {
                let data: Value = r.data;
                match data {
                    Value::Object(mut m) => {
                        m.insert("user_id".to_owned(), Value::Number(serde_json::Number::from(user_id.0)));
                        m.insert("guild_id".to_owned(), Value::Number(serde_json::Number::from(guild_id.0)));
                        Ok(Some(serde_json::from_value::<VoiceState>(Value::Object(m))?))
                    }
                    _ => Err(CacheError::WrongType()),
                }
            },
        }
    }

    async fn delete_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM voice_states WHERE "guild_id" = $1 AND "user_id" = $2;"#;
        sqlx::query(query).bind(guild_id.0 as i64).bind(user_id.0 as i64).execute(&self.pool).await.map_err(CacheError::DatabaseError)?;
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