use async_trait::async_trait;

use sqlx::{Error, PgPool};
use std::sync::Arc;

use crate::Table;

use futures::TryStreamExt;
use model::Snowflake;

pub struct WhitelabelGuilds {
    db: Arc<PgPool>,
}

#[async_trait]
impl Table for WhitelabelGuilds {
    async fn create_schema(&self) -> Result<(), Error> {
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS whitelabel_guilds(
	"bot_id" int8 NOT NULL,
	"guild_id" int8 NOT NULL,
	FOREIGN KEY("bot_id") REFERENCES whitelabel("bot_id") ON DELETE CASCADE ON UPDATE CASCADE,
	PRIMARY KEY("bot_id", "guild_id")
);
"#,
        )
        .execute(&*self.db)
        .await?;

        Ok(())
    }
}

impl WhitelabelGuilds {
    pub fn new(db: Arc<PgPool>) -> WhitelabelGuilds {
        WhitelabelGuilds { db }
    }

    pub async fn get_guilds(&self, bot_id: Snowflake) -> Result<Vec<Snowflake>, Error> {
        let query = r#"SELECT "guild_id" from whitelabel_guilds WHERE "bot_id" = $1;"#;

        let mut rows = sqlx::query_as::<_, (i64,)>(query)
            .bind(bot_id.0 as i64)
            .fetch(&*self.db);

        let mut guilds = Vec::new();
        while let Some(row) = rows.try_next().await? {
            guilds.push(Snowflake(row.0 as u64));
        }

        Ok(guilds)
    }

    pub async fn get_bot_by_guild(&self, guild_id: Snowflake) -> Result<Option<Snowflake>, Error> {
        let query = r#"SELECT "bot_id" from whitelabel_guilds WHERE "guild_id"=$1 LIMIT 1;"#;

        match sqlx::query_as::<_, (i64,)>(query)
            .bind(guild_id.0 as i64)
            .fetch_one(&*self.db)
            .await
        {
            Ok(id) => Ok(Some(Snowflake(id.0 as u64))),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn is_whitelabel_guild(&self, guild_id: Snowflake) -> Result<bool, Error> {
        let query = r#"SELECT 1 FROM whitelabel_guilds WHERE "guild_id"=$1 LIMIT 1;"#;

        match sqlx::query(query)
            .bind(guild_id.0 as i64)
            .fetch_one(&*self.db)
            .await
        {
            Ok(_) => Ok(true),
            Err(sqlx::Error::RowNotFound) => Ok(false),
            Err(e) => Err(e),
        }
    }

    pub async fn insert(&self, bot_id: Snowflake, guild_id: Snowflake) -> Result<(), Error> {
        let query = r#"INSERT INTO whitelabel_guilds("bot_id", "guild_id") VALUES($1, $2) ON CONFLICT("bot_id", "guild_id") DO NOTHING;"#;

        sqlx::query(query)
            .bind(bot_id.0 as i64)
            .bind(guild_id.0 as i64)
            .execute(&*self.db)
            .await?;

        Ok(())
    }

    pub async fn delete(&self, bot_id: Snowflake, guild_id: Snowflake) -> Result<(), Error> {
        let query = r#"DELETE FROM whitelabel_guilds WHERE "bot_id" = $1 AND "guild_id" = $2;"#;

        sqlx::query(query)
            .bind(bot_id.0 as i64)
            .bind(guild_id.0 as i64)
            .execute(&*self.db)
            .await?;

        Ok(())
    }
}
