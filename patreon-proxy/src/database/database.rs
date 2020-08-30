use crate::config::Config;
use tokio_postgres::{Client, NoTls, Error};

pub struct Database {
    client: Client
}

impl Database {
    pub async fn new(config: &Config) -> Result<Database, Error> {
        let (client, connection) = tokio_postgres::connect(&config.database_uri, NoTls).await?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                panic!("db connection error: {}", e);
            }
        });

        Ok(Database {
            client
        })
    }

    pub async fn create_schema(&self) -> Result<(), Error> {
        let query = "
CREATE TABLE IF NOT EXISTS patreon_keys(
    \"client_id\" VARCHAR(255) NOT NULL,
    \"access_token\" VARCHAR(255) NOT NULL,
    \"refresh_token\" VARCHAR(255) NOT NULL,
    \"expires\" TIMESTAMPTZ NOT NULL,
    PRIMARY KEY(\"client_id\")
);        
        ";

        let res = self.client.query(query, &[]).await?;
        Ok(())
    }
}