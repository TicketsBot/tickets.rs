use std::sync::Arc;

use sharder::{
    await_shutdown, build_redis, Config, RedisSessionStore, ShardManager, WhitelabelShardManager,
};

#[cfg(feature = "use-sentry")]
use sharder::setup_sentry;

use database::{sqlx::postgres::PgPoolOptions, Database};
use sharder::build_cache;

use jemallocator::Jemalloc;
use sharder::event_forwarding::HttpEventForwarder;
use tracing::info;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

// #[cfg(not(feature = "whitelabel"))]
// fn main() {
//     panic!("Started whitelabel sharder without whitelabel feature flag")
// }

// #[cfg(feature = "whitelabel")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_envvar();

    #[cfg(feature = "use-sentry")]
    let _guard = setup_sentry(&config);

    #[cfg(not(feature = "use-sentry"))]
    env_logger::init();

    // init db
    let db_opts = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(config.database_threads);
    let database = Arc::new(Database::connect(&config.database_uri[..], db_opts).await?);

    // init cache
    let cache = Arc::new(build_cache(&config).await);
    //cache.create_schema().await.unwrap();

    // init redis
    let redis = Arc::new(build_redis(&config));

    let session_store = RedisSessionStore::new(
        Arc::clone(&redis),
        "tickets:resume:whitelabel".to_string(),
        300,
    );

    let event_forwarder = Arc::new(HttpEventForwarder::new(
        HttpEventForwarder::build_http_client(),
    ));

    let sm = Arc::new(WhitelabelShardManager::new(
        config,
        database,
        cache,
        redis,
        session_store,
        event_forwarder,
    ));

    Arc::clone(&sm).connect().await;

    Arc::clone(&sm).listen_status_updates().await.unwrap();
    Arc::clone(&sm).listen_new_tokens().await.unwrap();
    Arc::clone(&sm).listen_delete().await.unwrap();

    await_shutdown()
        .await
        .expect("Failed to wait for shutdown signal");
    info!("Received shutdown signal");

    sm.shutdown().await;
    info!("Shard manager shutdown gracefully");

    Ok(())
}
