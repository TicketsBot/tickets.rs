use std::sync::Arc;
use crate::{Config, Result};
use std::net::SocketAddr;
use warp::Filter;

pub struct Server {
    pub config: Arc<Config>,
}

impl Server {
    pub fn new(config: Arc<Config>) -> Server {
        Server {
            config,
        }
    }

    pub async fn start(self: Arc<Self>) -> Result<()> {
        let address: SocketAddr = self.config.server_addr.clone().parse().unwrap();

        let filter = self.filter_handle();

        warp::serve(filter)
            .run(address)
            .await;

        Ok(())
    }

    fn filter_handle(self: Arc<Self>) -> impl Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        warp::post()
            .and(warp::path("handle"))
            .and(warp::any().map(move || self.clone()))
            .and(warp::body::json())
            .and_then(super::handle)
    }
}