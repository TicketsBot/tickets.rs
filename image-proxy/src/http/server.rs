use super::proxy::proxy;
use crate::Config;
use axum::http::{HeaderValue, Method};
use axum::routing::get;
use axum::{Extension, Router};
use hmac::digest::KeyInit;
use hmac::Hmac;
use log::info;
use sha2::Sha256;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::AllowMethods;
use tower_http::cors::{AllowOrigin, CorsLayer};

pub struct Server {
    config: Config,
    pub jwt_key: Hmac<Sha256>,
    pub http_client: reqwest::Client,
}

impl Server {
    pub fn new(config: Config) -> Server {
        let jwt_key = Hmac::new_from_slice(config.jwt_key.clone().as_bytes())
            .expect("Failed to parse HMAC key");

        Server {
            config,
            jwt_key,
            http_client: reqwest::ClientBuilder::new()
                .timeout(Duration::from_secs(3))
                .gzip(true)
                .resolve()
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    pub async fn start(self) {
        let server = Arc::new(self);

        let app = Router::new().route("/proxy", get(proxy)).layer(
            ServiceBuilder::new()
                .layer(Extension(server.clone()))
                .layer(CompressionLayer::new())
                .layer(
                    CorsLayer::new()
                        .allow_credentials(false)
                        .allow_methods(AllowMethods::list(vec![
                            Method::GET,
                            Method::HEAD,
                            Method::OPTIONS,
                        ]))
                        .allow_origin(AllowOrigin::exact(
                            HeaderValue::from_str("https://google.com").unwrap(),
                        )),
                ),
        );

        let server_addr = server
            .config
            .server_addr
            .parse()
            .expect("Failed to parse server address");

        info!("Starting server on {}", server_addr);

        axum::Server::bind(&server_addr)
            .serve(app.into_make_service())
            .await
            .unwrap();
    }
}
