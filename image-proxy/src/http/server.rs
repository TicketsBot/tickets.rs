use super::proxy::proxy;
use super::UsedTokenStore;
use crate::Config;
use axum::http::{HeaderValue, Method};
use axum::routing::get;
use axum::{Extension, Router};
use global_resolver::GlobalResolver;
use hmac::digest::KeyInit;
use hmac::Hmac;
use hyper::client::HttpConnector;
use hyper::Body;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use log::info;
use sha2::Sha256;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::cors::AllowMethods;
use tower_http::cors::{AllowOrigin, CorsLayer};

pub struct Server {
    config: Config,
    pub jwt_key: Hmac<Sha256>,
    pub used_token_store: UsedTokenStore,
    pub http_client: hyper::Client<HttpsConnector<HttpConnector<GlobalResolver>>, Body>,
}

impl Server {
    pub fn new(config: Config) -> Server {
        let jwt_key = Hmac::new_from_slice(config.jwt_key.clone().as_bytes())
            .expect("Failed to parse HMAC key");

        let mut http_connector = HttpConnector::new_with_resolver(GlobalResolver::new());
        http_connector.enforce_http(false);

        let https_connector = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .wrap_connector(http_connector);

        Server {
            config,
            jwt_key,
            used_token_store: UsedTokenStore::new(),
            http_client: hyper::Client::builder().build(https_connector),
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
