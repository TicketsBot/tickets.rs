use super::routes;
use crate::{Config, Database, Error};
use axum::routing::{get, post};
use axum::{Extension, Router};
use std::sync::Arc;

pub struct Server {
    pub config: Config,
    pub database: Database,
}

impl Server {
    pub fn new(config: Config, database: Database) -> Server {
        Server { config, database }
    }

    pub async fn start(self) -> Result<(), Error> {
        let server = Arc::new(self);

        let app = Router::new()
            .route("/", get(routes::index_handler))
            .route("/vote/dbl", post(routes::vote_dbl_handler))
            .layer(Extension(server.clone()));

        let addr = &server.config.server_addr[..].parse()?;

        hyper::Server::bind(addr)
            .serve(app.into_make_service())
            .await?;

        Ok(())
    }
}
