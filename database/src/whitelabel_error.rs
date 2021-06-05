use async_trait::async_trait;

use sqlx::{Error, PgPool};
use std::sync::Arc;

use crate::Table;

use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use model::Snowflake;

#[derive(sqlx::FromRow)]
pub struct WhitelabelError {
    pub error: String,
    pub time: DateTime<Utc>,
}

pub struct WhitelabelErrorTable {
    db: Arc<PgPool>,
}

#[async_trait]
impl Table for WhitelabelErrorTable {
    async fn create_schema(&self) -> Result<(), Error> {
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS whitelabel_errors(
	"error_id" serial,
	"user_id" int8 NOT NULL,
	"error" varchar(255) NOT NULL,
	"error_time" timestamptz NOT NULL,
	PRIMARY KEY("error_id")
);
"#,
        )
        .execute(&*self.db)
        .await?;

        Ok(())
    }
}

impl WhitelabelErrorTable {
    pub fn new(db: Arc<PgPool>) -> WhitelabelErrorTable {
        WhitelabelErrorTable { db }
    }

    pub async fn get_recent(
        &self,
        user_id: Snowflake,
        limit: i32,
    ) -> Result<Vec<WhitelabelError>, Error> {
        let query = r#"SELECT "error", "error_time" as "time" FROM whitelabel_errors WHERE "user_id" = $1 ORDER BY "error_id" DESC LIMIT $2;"#;

        let mut rows = sqlx::query_as::<_, WhitelabelError>(query)
            .bind(user_id.0 as i64)
            .bind(limit)
            .fetch(&*self.db);

        let mut errors = Vec::new();
        while let Some(err) = rows.try_next().await? {
            errors.push(err);
        }

        Ok(errors)
    }

    pub async fn append(&self, user_id: Snowflake, error: String) -> Result<(), Error> {
        let query = r#"INSERT INTO whitelabel_errors("user_id", "error", "error_time") VALUES($1, $2, NOW());"#;

        sqlx::query(query)
            .bind(user_id.0 as i64)
            .bind(error)
            .execute(&*self.db)
            .await?;

        Ok(())
    }
}
