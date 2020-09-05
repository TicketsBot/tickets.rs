use cache::*;
use sqlx::postgres::PgPoolOptions;
use model::user::User;
use model::{Snowflake, Discriminator};

#[tokio::main]
async fn main() -> Result<(), CacheError> {
    let uri = "postgres://ryan:ryan@localhost:5432/cache";
    let opts = Options::new(true, true, true, true, true, false, false);
    let pg_opts = PgPoolOptions::new().max_connections(16);

    let cache = PostgresCache::connect(uri, opts, pg_opts).await?;
    cache.create_schema().await?;

    cache.store_users(vec![&User {
        id: Snowflake(3),
        username: "monox".to_string(),
        discriminator: Discriminator(1111),
        avatar: None,
        bot: false,
        system: false,
        mfa_enabled: None,
        locale: None,
        verified: None,
        email: None,
        flags: None,
        premium_type: None,
        public_flags: None
    }, &User {
        id: Snowflake(2),
        username: "sex".to_string(),
        discriminator: Discriminator(222),
        avatar: None,
        bot: false,
        system: false,
        mfa_enabled: None,
        locale: None,
        verified: None,
        email: None,
        flags: None,
        premium_type: None,
        public_flags: None
    }]).await?;

    Ok(())
}
