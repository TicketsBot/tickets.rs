use cache::PostgresCache;
use log::info;
use server_counter::{http::Server, Config, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let config = Config::new();

    let cache = PostgresCache::connect(config.cache_uri.clone(), cache::Options::default(), 1)
        .await
        .map_err(Error::CacheError)?;

    let server = Server::new(config, cache);
    info!("Starting server...");
    server.start().await
}
