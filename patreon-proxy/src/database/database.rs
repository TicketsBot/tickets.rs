use super::Tokens;

use crate::config::Config;
use crate::error::PatreonError;
use tokio_postgres::{Client, NoTls};

use chrono::{DateTime, NaiveDateTime, Utc};

pub struct Database {
    client: Client,
}

impl Database {
    pub async fn new(config: &Config) -> Result<Database, tokio_postgres::Error> {
        let (client, connection) = tokio_postgres::connect(&config.database_uri, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                panic!("db connection error: {}", e);
            }
        });

        Ok(Database { client })
    }

    pub async fn create_schema(&self) -> Result<(), tokio_postgres::Error> {
        let query = "
CREATE TABLE IF NOT EXISTS patreon_keys(
    \"client_id\" VARCHAR(255) NOT NULL,
    \"access_token\" VARCHAR(255) NOT NULL,
    \"refresh_token\" VARCHAR(255) NOT NULL,
    \"expires\" TIMESTAMPTZ NOT NULL,
    PRIMARY KEY(\"client_id\")
);        
        ";

        self.client.query(query, &[]).await?;
        Ok(())
    }

    pub async fn get_tokens(&self, client_id: String) -> Result<Tokens, PatreonError> {
        let query = "
SELECT
    \"access_token\", \"refresh_token\", \"expires\"
FROM
    patreon_keys
WHERE
    \"client_id\" = $1
;
        ";

        let res = self.client.query(query, &[&client_id]).await?;
        if res.is_empty() {
            return PatreonError::MissingTokens(client_id).into();
        }

        let row = &res[0];
        let access_token: String = row.get(0);
        let refresh_token: String = row.get(1);
        let expires: DateTime<Utc> = row.get(2);

        Ok(Tokens::new(
            access_token,
            refresh_token,
            expires.timestamp(),
        ))
    }

    pub async fn update_tokens(
        &self,
        client_id: String,
        tokens: &Tokens,
    ) -> Result<(), tokio_postgres::Error> {
        let query = "
INSERT INTO
    patreon_keys
VALUES
    ($1, $2, $3, $4)
ON CONFLICT (\"client_id\") DO
UPDATE
    SET \"access_token\" = EXCLUDED.access_token,
        \"refresh_token\" = EXCLUDED.refresh_token,
        \"expires\" = EXCLUDED.expires
        ";

        let date_time =
            DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(tokens.expires, 0), Utc);
        self.client
            .query(
                query,
                &[
                    &client_id,
                    &tokens.access_token,
                    &tokens.refresh_token,
                    &date_time,
                ],
            )
            .await?;

        Ok(())
    }
}
