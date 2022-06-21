use async_trait::async_trait;

use sqlx::{Error, PgPool};
use std::sync::Arc;

use crate::Table;

use model::user::ActivityType;
use model::Snowflake;

pub struct WhitelabelStatus {
    db: Arc<PgPool>,
}

#[async_trait]
impl Table for WhitelabelStatus {
    async fn create_schema(&self) -> Result<(), Error> {
        sqlx::query(
            r#"
CREATE TABLE IF NOT EXISTS whitelabel_statuses(
	"bot_id" int8 UNIQUE NOT NULL,
	"status" varchar(255) NOT NULL,
	"status_type" int2 NOT NULL DEFAULT 2,
	FOREIGN KEY("bot_id") REFERENCES whitelabel("bot_id") ON DELETE CASCADE ON UPDATE CASCADE,
	PRIMARY KEY("bot_id")
);
"#,
        )
        .execute(&*self.db)
        .await?;

        Ok(())
    }
}

impl WhitelabelStatus {
    pub fn new(db: Arc<PgPool>) -> WhitelabelStatus {
        WhitelabelStatus { db }
    }

    pub async fn get(&self, bot_id: Snowflake) -> Result<(String, Option<ActivityType>), Error> {
        let query =
            r#"SELECT "status", "status_type" FROM whitelabel_statuses WHERE "bot_id" = $1;"#;

        let row = sqlx::query_as::<_, (String, i16)>(query)
            .bind(bot_id.0 as i64)
            .fetch_one(&*self.db)
            .await?;

        Ok((row.0, ActivityType::from_i16(row.1)))
    }

    pub async fn set(
        &self,
        bot_id: Snowflake,
        status: String,
        status_type: ActivityType,
    ) -> Result<(), Error> {
        let query = r#"
INSERT INTO whitelabel_statuses("bot_id", "status", "status_type")
VALUES($1, $2, $3)
ON CONFLICT("bot_id") DO UPDATE SET "status" = $2, "status_type" = $3;
"#;

        sqlx::query(query)
            .bind(bot_id.0 as i64)
            .bind(status)
            .bind(status_type as i16)
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
