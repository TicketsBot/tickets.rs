use super::proxy;
use crate::Config;
use axum::routing::post;
use axum::{Extension, Router};
use global_resolver::GlobalResolver;
use hyper::client::HttpConnector;
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use log::info;
use std::sync::Arc;

pub(crate) type HttpClient<R = GlobalResolver> = hyper::Client<HttpsConnector<HttpConnector<R>>>;

pub struct Server {
    pub(crate) config: Config,
    pub(crate) http_client: HttpClient<GlobalResolver>,
    #[cfg(feature = "pre-resolve")]
    pub(crate) resolver: GlobalResolver,
}

impl Server {
    pub fn new(config: Config) -> Self {
        let mut http_connector = HttpConnector::new_with_resolver(GlobalResolver::new());
        http_connector.enforce_http(false);

        let https_connector = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .wrap_connector(http_connector);

        let http_client = hyper::Client::builder().build(https_connector);

        Self::new_with_client(config, http_client)
    }

    pub fn new_with_client(config: Config, http_client: HttpClient<GlobalResolver>) -> Self {
        Self {
            config,
            http_client,
            #[cfg(feature = "pre-resolve")]
            resolver: GlobalResolver::new(),
        }
    }

    pub async fn start(self) {
        let server = Arc::new(self);

        let app = Router::new()
            .route("/proxy", post(proxy))
            .layer(Extension(server.clone()));

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
