use cache::{Options, PostgresCache};
use cache_sync_service::{processor::Manager, Config, Result};
use common::prometheus_server;
use tokio::signal::ctrl_c;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::from_env()?;

    prometheus_server::start_server(config.metric_server_addr.as_str())
        .await
        .expect("Failed to start metrics server");

    info!("Connecting to Postgres...");
    let cache = connect_postgres(&config).await?;

    info!(workers = %config.workers, "Starting workers...");
    let manager = Manager::new(config, cache);
    manager.start()?;

    ctrl_c().await.expect("Failed to listen for ctrl-c");

    Ok(())
}

async fn connect_postgres(config: &Config) -> Result<PostgresCache> {
    let opts = Options::new(true, true, true, true, true, true, false, false);
    PostgresCache::connect(config.postgres_uri.clone(), opts, config.workers)
        .await
        .map_err(Into::into)
}
