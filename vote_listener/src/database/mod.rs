use crate::{Config, Error};
use bb8::Pool;
use bb8_postgres::tokio_postgres::NoTls;
use bb8_postgres::PostgresConnectionManager;
use model::Snowflake;

pub type ConnectionPool = Pool<PostgresConnectionManager<NoTls>>;

pub struct Database {
    pool: ConnectionPool,
}

impl Database {
    pub async fn connect(config: &Config) -> Result<Self, Error> {
        let manager =
            PostgresConnectionManager::new_from_stringlike(config.database_uri.as_str(), NoTls)?;

        let pool = Pool::builder().max_size(1).build(manager).await?;
        Ok(Self { pool })
    }

    pub async fn add_vote(&self, user_id: Snowflake) -> Result<(), Error> {
        let query = r#"
INSERT INTO
    votes("user_id", "vote_time")
VALUES
    ($1, NOW())
ON CONFLICT("user_id") DO
    UPDATE SET "vote_time" = NOW()
;"#;

        let conn = self.pool.get().await?;
        conn.execute(query, &[&(user_id.0 as i64)]).await?;

        Ok(())
    }
}
