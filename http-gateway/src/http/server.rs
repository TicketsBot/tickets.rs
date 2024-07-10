use crate::http::response::ErrorResponse;
use crate::{Config, Error};
use database::Database;
use ed25519_dalek::Signature;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use warp::http::StatusCode;
use warp::reply::Json;
use warp::{Filter, Rejection};

use cache::Cache;

pub struct Server<T: Cache> {
    pub config: Config,
    pub database: Database,
    pub cache: T,
    pub http_client: reqwest::Client,
}

impl<T: Cache> Server<T> {
    pub fn new(config: Config, database: Database, cache: T) -> Server<T> {
        Server {
            config,
            database,
            cache,
            http_client: Self::build_http_client(),
        }
    }

    pub async fn start(self) -> Result<(), Error> {
        let address: SocketAddr = self.config.server_addr.clone().parse().unwrap();

        let filter = Arc::new(self).filter_handle();

        warp::serve(filter).run(address).await;

        Ok(())
    }

    fn filter_handle(
        self: Arc<Self>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::post()
            .and(warp::path("handle"))
            .and(warp::path::param())
            .and(warp::any().map(move || self.clone()))
            .and(Self::parse_signature())
            .and(warp::header("x-signature-timestamp"))
            .and(warp::body::bytes())
            .and_then(super::handle)
            .map(|reply| warp::reply::with_header(reply, "Content-Type", "application/json"))
            .with(warp::log("warp"))
            .recover(|error: Rejection| async move {
                eprintln!("Rejecting with {:?}", error);

                if let Some(err) = error.find::<Error>() {
                    let err_response: ErrorResponse = err.into();
                    let json: Json = err_response.into();

                    let status_code = match err {
                        Error::InvalidSignature(..)
                        | Error::InvalidSignatureLength
                        | Error::InvalidSignatureFormat(..) => StatusCode::UNAUTHORIZED,
                        Error::MissingGuildId => StatusCode::BAD_REQUEST,
                        _ => StatusCode::INTERNAL_SERVER_ERROR,
                    };

                    Ok(warp::reply::with_status(json, status_code))
                } else {
                    Err(error)
                }
            })
    }

    fn parse_signature() -> impl Filter<Extract = (Signature,), Error = warp::Rejection> + Clone {
        warp::header("x-signature-ed25519").and_then(|signature: String| async move {
            let mut bytes = [0u8; 64];
            if let Err(e) = hex::decode_to_slice(signature, &mut bytes) {
                return Err(warp::reject::custom(Error::InvalidSignatureFormat(e)));
            }

            let signature = match Signature::from_bytes(&bytes) {
                Ok(signature) => signature,
                Err(e) => return Err(warp::reject::custom(Error::InvalidSignature(e))),
            };

            Ok(signature)
        })
    }

    fn build_http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .gzip(true)
            .build()
            .expect("build_http_client")
    }
}
