use crate::{CacheError, Options, Result};
use tracing::{error, info};

#[cfg(feature = "metrics")]
use lazy_static::lazy_static;

use crate::postgres::worker::Worker;
#[cfg(feature = "metrics")]
use prometheus::{register_histogram_vec, HistogramVec};
use tokio_postgres::tls::NoTlsStream;
use tokio_postgres::{Connection, NoTls, Socket};

use backoff::ExponentialBackoff;

#[cfg(feature = "metrics")]
lazy_static! {
    static ref HISTOGRAM: HistogramVec =
        register_histogram_vec!("cache_timings", "Cache Timings", &["table"])
            .expect("Failed to register cache timings histogram");
}

pub struct PostgresCache {}

impl PostgresCache {
    /// panics if URI is invalid
    pub async fn connect(uri: String, opts: Options, workers: usize) -> Result<PostgresCache> {
        info!(worker_count = workers, options = ?opts, "Connecting to database");

        // start workers
        for id in 0..workers {
            let uri = uri.clone();

            // run executor in background
            tokio::spawn(async move {
                // Loop to reconnect after conn dies
                loop {
                    let _: Result<()> =
                        backoff::future::retry(ExponentialBackoff::default(), || async {
                            info!(id, "Starting cache worker");
                            let (worker, conn) = Self::spawn_worker(
                                id,
                                opts.clone(),
                                &uri[..],
                            )
                            .await?;
                            info!(id, "Cache worker started and connected");

                            if let Err(e) = conn.await {
                                error!(id, error = %e, "Failed to connect to database");
                                return Err(backoff::Error::Transient(CacheError::DatabaseError(
                                    e,
                                )));
                            }

                            info!(id, "Cache worker disconnected");

                            Err(backoff::Error::Transient(CacheError::Disconnected))
                        })
                        .await;
                }
            });
        }

        Ok(PostgresCache {})
    }

    #[tracing::instrument(name = "spawn_worker", skip(uri))]
    async fn spawn_worker(
        id: usize,
        opts: Options,
        uri: &str,
    ) -> Result<(Worker, Connection<Socket, NoTlsStream>)> {
        let (client, conn) = tokio_postgres::connect(uri, NoTls)
            .await
            .map_err(CacheError::DatabaseError)?;

        let worker = Worker::new(id, opts, client);

        Ok((worker, conn))
    }
}
