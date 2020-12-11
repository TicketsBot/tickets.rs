use http_gateway::{Config, Error};
use http_gateway::http;
use deadpool_redis::Pool;
use deadpool_redis::Config as RedisConfig;
use deadpool::managed::PoolConfig;
use database::Database;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = Config::from_envvar();

    let redis = connect_redis(&config);

    let db_opts = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(config.database.threads);

    let db = Database::connect(&*config.database.uri, db_opts).await.map_err(Error::DatabaseError)?;

    let server = http::Server::new(config, redis, db);
    server.start().await
}

fn connect_redis(config: &Config) -> Pool {
    let mut cfg = RedisConfig::default();
    cfg.url = Some(format!("redis://{}/", config.redis.address));
    cfg.pool = Some(PoolConfig::new(config.redis.threads));

    cfg.create_pool().unwrap()
}