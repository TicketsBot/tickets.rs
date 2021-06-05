use database::Database;
use http_gateway::http;
use http_gateway::{Config, Error};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = Config::from_envvar();

    let db_opts = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(config.database.threads);

    let db = Database::connect(&*config.database.uri, db_opts)
        .await
        .map_err(Error::DatabaseError)?;

    let server = http::Server::new(config, db);
    server.start().await
}
