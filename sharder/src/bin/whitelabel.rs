use std::sync::Arc;

use sharder::{ShardManager, build_redis, WhitelabelShardManager};

use sharder::{var_or_panic, build_cache};
use database::Database;
use sqlx::postgres::PgPoolOptions;

use jemallocator::Jemalloc;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let sm = WhitelabelShardManager::new(
        sharder_count,
        sharder_id,
        database,
        cache,
        redis
    ).await;

    Arc::clone(&sm).connect().await;

    Arc::clone(&sm).listen_status_updates().await.unwrap();
    Arc::clone(&sm).listen_new_tokens().await.unwrap();
    Arc::clone(&sm).listen_delete().await.unwrap();
    sm.start_error_loop().await;

    Ok(())
}
