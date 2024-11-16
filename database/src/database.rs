use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;

use crate::{Table, Whitelabel, WhitelabelErrorTable, WhitelabelGuilds, WhitelabelStatus};

pub struct Database {
    pub whitelabel: Whitelabel,
    pub whitelabel_errors: WhitelabelErrorTable,
    pub whitelabel_guilds: WhitelabelGuilds,
    pub whitelabel_status: WhitelabelStatus,
}

impl Database {
    pub async fn connect(uri: &str, pg_opts: PgPoolOptions) -> Result<Database, sqlx::Error> {
        let pool = Arc::new(pg_opts.connect(uri).await?);

        Ok(Database {
            whitelabel: Whitelabel::new(Arc::clone(&pool)),
            whitelabel_errors: WhitelabelErrorTable::new(Arc::clone(&pool)),
            whitelabel_guilds: WhitelabelGuilds::new(Arc::clone(&pool)),
            whitelabel_status: WhitelabelStatus::new(Arc::clone(&pool)),
        })
    }

    pub async fn create_schema(&self) -> Result<(), sqlx::Error> {
        self.whitelabel.create_schema().await?;
        self.whitelabel_errors.create_schema().await?;
        self.whitelabel_guilds.create_schema().await?;
        self.whitelabel_status.create_schema().await?;

        Ok(())
    }
}
