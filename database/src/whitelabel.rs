use async_trait::async_trait;

use sqlx::{Error, PgPool};
use std::sync::Arc;

use crate::Table;

use futures::TryStreamExt;
use model::Snowflake;

#[derive(sqlx::FromRow, Debug)]
pub struct WhitelabelBot {
    pub user_id: i64,
    pub bot_id: i64,
    pub public_key: String,
    pub token: String,
}

pub struct Whitelabel {
    db: Arc<PgPool>,
}

#[async_trait]
impl Table for Whitelabel {
    async fn create_schema(&self) -> Result<(), Error> {
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS whitelabel(
	"user_id" int8 UNIQUE NOT NULL,
	"bot_id" int8 UNIQUE NOT NULL,
    "public_key" CHAR(64) NOT NULL,
	"token" VARCHAR(84) NOT NULL UNIQUE,
	PRIMARY KEY("user_id")
);
CREATE INDEX IF NOT EXISTS whitelabel_bot_id ON whitelabel("bot_id");
        "#,
        )
        .execute(&*self.db)
        .await?;

        Ok(())
    }
}

impl Whitelabel {
    pub fn new(db: Arc<PgPool>) -> Whitelabel {
        Whitelabel { db }
    }

    pub async fn get_user_by_id(&self, user_id: Snowflake) -> Result<Option<WhitelabelBot>, Error> {
        let query = r#"SELECT "user_id", "bot_id", "public_key", "token" FROM whitelabel WHERE "user_id" = $1"#;

        match sqlx::query_as::<_, WhitelabelBot>(query)
            .bind(user_id.0 as i64)
            .fetch_one(&*self.db)
            .await
        {
            Ok(bot) => Ok(Some(bot)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn get_bot_by_id(&self, bot_id: Snowflake) -> Result<Option<WhitelabelBot>, Error> {
        let query = r#"SELECT "user_id", "bot_id", "public_key", "token" FROM whitelabel WHERE "bot_id" = $1"#;

        match sqlx::query_as::<_, WhitelabelBot>(query)
            .bind(bot_id.0 as i64)
            .fetch_one(&*self.db)
            .await
        {
            Ok(bot) => Ok(Some(bot)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn get_bot_by_token(&self, token: &str) -> Result<Option<WhitelabelBot>, Error> {
        let query = r#"SELECT "user_id", "bot_id", "public_key", "token" FROM whitelabel WHERE "token" = $1"#;

        match sqlx::query_as::<_, WhitelabelBot>(query)
            .bind(token)
            .fetch_one(&*self.db)
            .await
        {
            Ok(bot) => Ok(Some(bot)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn get_bots_by_sharder(
        &self,
        sharder_count: u16,
        sharder_id: u16,
    ) -> Result<Vec<WhitelabelBot>, Error> {
        let query = r#"SELECT "user_id", "bot_id", "public_key", "token" FROM whitelabel WHERE "bot_id" % $1 = $2"#;

        let mut rows = sqlx::query_as::<_, WhitelabelBot>(query)
            .bind(sharder_count as i32)
            .bind(sharder_id as i32)
            .fetch(&*self.db);

        let mut bots = Vec::new();
        while let Some(row) = rows.try_next().await? {
            bots.push(row);
        }

        Ok(bots)
    }

    pub async fn insert(&self, bot: WhitelabelBot) -> Result<(), Error> {
        let query = r#"
INSERT INTO whitelabel
    ("user_id", "bot_id", "public_key", "token")
VALUES
    ($1, $2, $3, $4)
ON CONFLICT("user_id") DO
    UPDATE
        SET "bot_id" = $2,
            "public_key" = $3,
            "token" = $4";
"#;

        sqlx::query(query)
            .bind(bot.user_id)
            .bind(bot.bot_id)
            .bind(bot.public_key)
            .bind(bot.token)
            .execute(&*self.db)
            .await?;

        Ok(())
    }

    pub async fn delete_by_user_id(&self, user_id: Snowflake) -> Result<(), Error> {
        let query = r#"DELETE FROM whitelabel WHERE "user_id" = $1;"#;

        sqlx::query(query)
            .bind(user_id.0 as i64)
            .execute(&*self.db)
            .await?;

        Ok(())
    }

    pub async fn delete_by_bot_id(&self, bot_id: Snowflake) -> Result<(), Error> {
        let query = r#"DELETE FROM whitelabel WHERE "bot_id" = $1;"#;

        sqlx::query(query)
            .bind(bot_id.0 as i64)
            .execute(&*self.db)
            .await?;

        Ok(())
    }

    pub async fn delete_by_token(&self, token: &str) -> Result<(), Error> {
        let query = r#"DELETE FROM whitelabel WHERE "token" = $1;"#;

        sqlx::query(query).bind(token).execute(&*self.db).await?;

        Ok(())
    }
}
