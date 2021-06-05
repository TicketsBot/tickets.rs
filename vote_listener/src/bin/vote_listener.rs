use std::sync::Arc;
use vote_listener::{http::Server, Config, Database, Error};

#[actix_web::main]
async fn main() -> Result<(), Error> {
    let config = Config::new();

    let database = Database::connect(&config).await?;

    let server = Server::new(config, database);
    Arc::new(server).start().await
}
