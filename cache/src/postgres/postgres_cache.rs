use std::str::FromStr;

use crate::{Options, Result};
use tracing::info;

#[cfg(feature = "metrics")]
use lazy_static::lazy_static;

#[cfg(feature = "metrics")]
use prometheus::{register_histogram_vec, HistogramVec};
use deadpool_postgres::{tokio_postgres::{Config, NoTls}, Manager, ManagerConfig, Pool, RecyclingMethod};

use super::worker::Worker;

#[cfg(feature = "metrics")]
lazy_static! {
    static ref HISTOGRAM: HistogramVec =
        register_histogram_vec!("cache_timings", "Cache Timings", &["table"])
            .expect("Failed to register cache timings histogram");
}

pub struct PostgresCache {
    options: Options,
    pool: Pool,
}

impl PostgresCache {
    pub async fn connect(uri: String, options: Options, workers: usize) -> Result<PostgresCache> {
        info!(worker_count = workers, options = ?options, "Connecting to database");

        let postgres_cfg = Config::from_str(uri.as_str())?;
        let manager_cfg = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };
        let manager = Manager::from_config(postgres_cfg, NoTls, manager_cfg);
        let pool = Pool::builder(manager)
            .max_size(workers)
            .build()?;

        Ok(PostgresCache {
            options,
            pool,
        })
    }

    pub async fn build_worker(&self) -> Result<Worker> {
        let conn = self.pool.get().await?;
        Ok(Worker::new(self.options, conn))
    }
}
