use super::Tokens;

use crate::config::Config;
use crate::error::{Error, Result};
use tokio_postgres::NoTls;

use chrono::{DateTime, NaiveDateTime, Utc};
use deadpool_postgres::{Config as ConnectionConfig, GenericClient, Pool, PoolConfig, Runtime};
use url::Url;

pub struct Database {
    pool: Pool,
}

impl Database {
    pub async fn connect(config: &Config) -> Result<Database> {
        let url = Url::parse(&config.database_uri)?;

        let pool_size = url
            .query_pairs()
            .find(|(k, _)| k == "max_size")
            .map(|(_, v)| v.parse::<usize>())
            .unwrap_or(Ok(4))?;

        let mut pool_config = ConnectionConfig::new();
        pool_config.host = url.host_str().map(|s| s.to_owned());
        pool_config.port = url.port();
        pool_config.dbname = Some(url.path().trim_start_matches('/').to_owned());
        pool_config.user = Some(url.username().to_owned());
        pool_config.password = url.password().map(|s| s.to_owned());
        pool_config.pool = Some(PoolConfig::new(pool_size));

        let pool = pool_config.create_pool(Some(Runtime::Tokio1), NoTls)?;

        Ok(Database { pool })
    }

    pub async fn create_schema(&self) -> Result<()> {
        let query = include_str!("sql/patreon_keys/schema.sql");
        self.pool.get().await?.execute(query, &[]).await?;

        Ok(())
    }

    pub async fn get_tokens(&self, client_id: String) -> Result<Tokens> {
        let query = include_str!("sql/patreon_keys/get_tokens.sql");

        let conn = self.pool.get().await?;
        let statement = conn.prepare_cached(query).await?;
        let rows = conn.query(&statement, &[&client_id]).await?;

        if rows.is_empty() {
            return Error::MissingTokens(client_id).into();
        }

        let row = &rows[0];

        let access_token: String = row.try_get(0)?;
        let refresh_token: String = row.try_get(1)?;
        let expires: DateTime<Utc> = row.try_get(2)?;

        Ok(Tokens::new(
            access_token,
            refresh_token,
            expires.timestamp(),
        ))
    }

    pub async fn update_tokens(&self, client_id: String, tokens: &Tokens) -> Result<()> {
        let query = include_str!("sql/patreon_keys/insert_tokens.sql");
        let date_time =
            DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(tokens.expires, 0), Utc);

        let conn = self.pool.get().await?;
        let statement = conn.prepare_cached(query).await?;
        conn.execute(
            &statement,
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
