use actix_web::{App, HttpServer};
use crate::{Error, Config, Database};
use super::routes;
use std::sync::Arc;

pub struct Server {
    pub config: Config,
    pub database: Database,
}

impl Server {
    pub fn new(config: Config, database: Database) -> Server {
        return Server {
            config,
            database,
        };
    }

    pub async fn start(self: Arc<Self>) -> Result<(), Error> {
        let server = self.clone();
        let factory = move || {
            App::new()
                .data(server.clone())
                .service(routes::index_handler)
                .service(routes::vote_dbl_handler)
        };

        let server = HttpServer::new(factory)
            .bind(&**&self.config.server_addr)
            .map_err(Error::IOError)?;

        server.run().await.map_err(Error::IOError)
    }
}

