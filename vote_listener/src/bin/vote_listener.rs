use log::info;
use vote_listener::{http::Server, Config, Database, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::init();

    let config = Config::new();

    let database = Database::connect(&config).await?;

    let server = Server::new(config, database);
    info!("Starting server...");
    server.start().await
}
