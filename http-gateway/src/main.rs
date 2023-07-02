use cache::PostgresCache;
use database::Database;
use http_gateway::http;
use http_gateway::{Config, Error};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = Config::from_envvar();

    let db_opts = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(config.database_threads);

    let db = Database::connect(&*config.database_uri, db_opts)
        .await
        .map_err(Error::DatabaseError)?;

    let cache_opts = cache::Options {
        users: true,
        guilds: false,
        members: true,
        channels: false,
        threads: false,
        roles: false,
        emojis: false,
        voice_states: false,
    };

    let cache = PostgresCache::connect(config.cache_uri.clone(), cache_opts, config.cache_threads)
        .await
        .map_err(Error::CacheError)?;

    let server = http::Server::new(config, db, cache);
    server.start().await
}
