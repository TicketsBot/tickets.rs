mod cache;
pub use cache::Cache;

mod options;
pub use options::Options;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "postgres")]
pub use postgres::PostgresCache;
#[cfg(feature = "postgres")]
pub use deadpool_postgres::tokio_postgres;

#[cfg(feature = "memory")]
mod memory;
#[cfg(feature = "memory")]
pub use memory::*;

mod error;
pub use error::{CacheError, Result};

#[cfg(feature = "cache-model")]
pub mod model;
