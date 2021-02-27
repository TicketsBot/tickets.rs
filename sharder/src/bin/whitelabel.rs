use std::sync::Arc;

use sharder::{ShardManager, build_redis, WhitelabelShardManager, Config};

use sharder::{var_or_panic, build_cache};
use database::{Database, sqlx::postgres::PgPoolOptions};

use tokio::signal;

use jemallocator::Jemalloc;
use sharder::event_forwarding::HttpEventForwarder;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[cfg(not(feature = "whitelabel"))]
fn main() {
    panic!("Started whitelabel sharder without whitelabel feature flag")
}

#[cfg(feature = "whitelabel")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(Config::from_envvar());

    let sharder_id: u16 = var_or_panic("SHARDER_ID").parse().unwrap();
    let sharder_count: u16 = var_or_panic("SHARDER_TOTAL").parse().unwrap();

    // init db
    let db_opts = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(var_or_panic("DATABASE_THREADS").parse().unwrap());
    let database = Arc::new(Database::connect(&var_or_panic("DATABASE_URI"), db_opts).await?);

    // init cache
    let cache = Arc::new(build_cache().await);
    //cache.create_schema().await.unwrap();

    // init redis
    let redis = Arc::new(build_redis());

    let event_forwarder = Arc::new(HttpEventForwarder::new(HttpEventForwarder::build_http_client()));
    Arc::clone(&event_forwarder).start_reset_cookie_loop();

    let sm = Arc::new(WhitelabelShardManager::new(
        config,
        sharder_count,
        sharder_id,
        database,
        cache,
        redis,
        event_forwarder,
    ));

    Arc::clone(&sm).connect().await;

    Arc::clone(&sm).listen_status_updates().await.unwrap();
    Arc::clone(&sm).listen_new_tokens().await.unwrap();
    Arc::clone(&sm).listen_delete().await.unwrap();

    Ok(signal::ctrl_c().await?)
}
