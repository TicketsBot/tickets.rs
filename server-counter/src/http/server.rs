use super::routes;
use crate::{Config, Error};
use axum::handler::get;
use axum::{AddExtensionLayer, Router};
use cache::Cache;
use log::error;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;

pub struct Server<T: Cache> {
    pub config: Config,
    pub cache: T,
    pub count: RwLock<usize>,
}

impl<T: Cache> Server<T> {
    pub fn new(config: Config, cache: T) -> Server<T> {
        Server {
            config,
            cache,
            count: RwLock::new(0),
        }
    }

    pub async fn start(self) -> Result<(), Error> {
        let server = Arc::new(self);

        server.clone().start_update_loop();

        let app = Router::new()
            .route("/total", get(routes::total_handler::<T>))
            .layer(AddExtensionLayer::new(server.clone()))
            .route("/total/prometheus", get(routes::prometheus_handler::<T>))
            .layer(AddExtensionLayer::new(server.clone()));

        let addr = &server.config.server_addr[..].parse()?;

        hyper::Server::bind(addr)
            .serve(app.into_make_service())
            .await?;

        Ok(())
    }

    fn start_update_loop(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                let count = match self.cache.get_guild_count().await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("Error while getting guild count: {}", e);
                        continue;
                    }
                };

                {
                    *self.count.write() = count;
                }

                tokio::time::sleep(Duration::from_secs(15)).await;
            }
        });
    }
}
