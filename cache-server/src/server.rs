use std::net::SocketAddr;
use std::str::FromStr;
use crate::endpoints;
use axum::handler::get;
use axum::{AddExtensionLayer, Router};
use cache::MemoryCache;
use std::sync::Arc;
use axum::service::post;

pub struct Server {
    cache: Arc<MemoryCache>,
}

impl Server {
    pub fn new(cache: Arc<MemoryCache>) -> Self {
        Server { cache }
    }

    pub async fn start(&self, server_addr: &str) {
        let app = Router::new()
            .route("/guilds/:guild_id", get(endpoints::get_guild))
            .route("/guilds/:guild_id/channels", get(endpoints::get_guild_channels))
            .route("/guilds/:guild_id/channels/:channel_id", get(endpoints::get_guild_channel))

            .route("/guilds/:guild_id/roles", get(endpoints::get_guild_roles))
            .route("/guilds/:guild_id/roles/:role_id", get(endpoints::get_role))

            .route("/guilds/:guild_id/members", post(endpoints::post_members))
            .route("/guilds/:guild_id/members/:user_id", get(endpoints::get_member).post(endpoints::post_member))

            .route("/users/", post(endpoints::post_users))
            .route("/users/:user_id", get(endpoints::get_member).post(endpoints::post_user))

            .route("/channels/:channel_id", get(endpoints::get_dm_channel))

            .layer(AddExtensionLayer::new(self.cache.clone()));

        let server_addr = SocketAddr::from_str(server_addr).expect("Failed to convert to SocketAddr");

        hyper::Server::bind(&server_addr)
            .serve(app.into_make_service())
            .await;
    }
}
