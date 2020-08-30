mod config;
mod database;

use config::Config;
use database::Database;

use tokio_postgres::{Error};

#[tokio::main]
pub async fn main() -> Result<(), Error> {
    let config = Config::new().unwrap();

    let db_client = Database::new(&config).await?;
    db_client.create_schema().await?;

    Ok(())
}
