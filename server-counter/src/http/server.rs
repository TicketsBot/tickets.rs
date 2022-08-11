use super::routes;
use crate::{Config, Error};
use axum::routing::get;
use axum::{Extension, Router};
use cache::Cache;
use log::error;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

pub struct Server<T: Cache> {
    pub config: Config,
    pub cache: T,
    pub count: AtomicUsize,
}

impl<T: Cache> Server<T> {
    pub fn new(config: Config, cache: T) -> Server<T> {
        Server {
            config,
            cache,
            count: AtomicUsize::new(0),
        }
    }

    pub async fn start(self) -> Result<(), Error> {
        let server = Arc::new(self);

        server.clone().start_update_loop();

        let app = Router::new()
            .route("/total", get(routes::total_handler::<T>))
            .route("/total/prometheus", get(routes::prometheus_handler::<T>))
            .layer(Extension(server.clone()));

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

                self.count.store(count, Ordering::Relaxed);

                sleep(Duration::from_secs(15)).await;
            }
        });
    }
}
