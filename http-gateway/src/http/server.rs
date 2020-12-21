use std::sync::Arc;
use crate::{Config, Error};
use std::net::SocketAddr;
use warp::{Filter, Rejection};
use ed25519_dalek::Signature;
use warp::http::StatusCode;
use crate::http::response::ErrorResponse;
use warp::reply::Json;
use database::Database;
use std::time::Duration;

pub struct Server {
    pub config: Config,
    pub database: Database,
    pub http_client: reqwest::Client,
}

impl Server {
    pub fn new(config: Config, database: Database) -> Server {
        Server {
            config,
            database,
            http_client: Server::build_http_client(),
        }
    }

    pub async fn start(self) -> Result<(), Error> {
        let address: SocketAddr = self.config.server_addr.clone().parse().unwrap();

        let filter = Arc::new(self).filter_handle();

        warp::serve(filter)
            .run(address)
            .await;

        Ok(())
    }

    fn filter_handle(self: Arc<Self>) -> impl Filter<Extract=impl warp::Reply, Error=warp::Rejection> + Clone {
        warp::post()
            .and(warp::path("handle"))
            .and(warp::path::param())
            .and(warp::any().map(move || self.clone()))
            .and(Server::parse_signature())
            .and(warp::header("x-signature-timestamp"))
            .and(warp::body::bytes())
            .and_then(super::handle)
            .with(warp::log("warp"))
            .recover(|error: Rejection| async move {
                if let Some(err) = error.find::<Error>() {
                    let json: Json = ErrorResponse::from(&err).into();

                    let status_code = match err {
                        Error::InvalidSignature(..) |
                        Error::InvalidSignatureLength |
                        Error::InvalidSignatureFormat(..) => StatusCode::UNAUTHORIZED,
                        Error::MissingGuildId => StatusCode::BAD_REQUEST,
                        _ => StatusCode::INTERNAL_SERVER_ERROR
                    };

                    Ok(warp::reply::with_status(json, status_code))
                } else {
                    Err(error)
                }
            })
    }

    fn parse_signature() -> impl Filter<Extract=(Signature, ), Error=warp::Rejection> + Clone {
        warp::header("x-signature-ed25519").and_then(|signature: String| async move {
            let mut bytes = [0u8; 64];
            if let Err(e) = hex::decode_to_slice(signature, &mut bytes) {
                return Err(warp::reject::custom(Error::InvalidSignatureFormat(e)));
            }

            Ok(Signature::new(bytes.into()))
        })
    }

    fn build_http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .gzip(true)
            .http2_prior_knowledge()
            .build()
            .expect("build_http_client")
    }
}