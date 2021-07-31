use crate::{Config, Error};
use model::Snowflake;
use tokio_postgres::{Client, NoTls};

pub struct Database {
    client: Client,
}

impl Database {
    pub async fn connect(config: &Config) -> Result<Database, Error> {
        let (client, connection) = tokio_postgres::connect(&*config.database_uri, NoTls)
            .await
            .map_err(Error::DatabaseError)?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                panic!("db connection error: {}", e);
            }
        });

        Ok(Database { client })
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

        self.client
            .execute(query, &[&(user_id.0 as i64)])
            .await
            .map_err(Error::DatabaseError)?;

        Ok(())
    }
}
