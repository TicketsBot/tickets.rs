use async_trait::async_trait;

use sqlx::{PgPool, Error};
use std::sync::Arc;

use crate::Table;

use model::Snowflake;

pub struct WhitelabelStatus {
    db: Arc<PgPool>,
}

#[async_trait]
impl Table for WhitelabelStatus {
    async fn create_schema(&self) -> Result<(), Error> {
        sqlx::query(r#"
CREATE TABLE IF NOT EXISTS whitelabel_statuses(
	"bot_id" int8 UNIQUE NOT NULL,
	"status" varchar(255) NOT NULL,
	FOREIGN KEY("bot_id") REFERENCES whitelabel("bot_id") ON DELETE CASCADE ON UPDATE CASCADE,
	PRIMARY KEY("bot_id")
);
"#).execute(&*self.db).await?;

        Ok(())
    }
}

impl WhitelabelStatus {
    pub fn new(db: Arc<PgPool>) -> WhitelabelStatus {
        WhitelabelStatus { db }
    }

    pub async fn get(&self, bot_id: Snowflake) -> Result<String, Error> {
        let query = r#"SELECT "status" FROM whitelabel_statuses WHERE "bot_id" = $1;"#;

        let row = sqlx::query_as::<_, (String,)>(query)
            .bind(bot_id.0 as i64)
            .fetch_one(&*self.db)
            .await?;

        Ok(row.0)
    }

    pub async fn set(&self, bot_id: Snowflake, status: String) -> Result<(), Error> {
        let query = r#"INSERT INTO whitelabel_statuses("bot_id", "status") VALUES($1, $2) ON CONFLICT("bot_id") DO UPDATE SET "status" = $2;"#;

        sqlx::query(query)
            .bind(bot_id.0 as i64)
            .bind(status)
            .execute(&*self.db)
            .await?;

        Ok(())
    }

    pub async fn delete(&self, bot_id: Snowflake) -> Result<(), Error> {
        let query = r#"DELETE FROM whitelabel_statuses WHERE "bot_id" = $1;"#;

        sqlx::query(query)
            .bind(bot_id.0 as i64)
            .execute(&*self.db)
            .await?;

        Ok(())
    }
}