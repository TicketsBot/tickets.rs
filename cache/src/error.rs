pub type Result<T> = std::result::Result<T, CacheError>;

#[derive(thiserror::Error, Debug)]
pub enum CacheError {
    #[cfg(feature = "postgres")]
    #[error("Error occurred while interacting with DB: {0}")]
    DatabaseError(#[from] deadpool_postgres::tokio_postgres::Error),

    #[cfg(feature = "postgres")]
    #[error("Error occurred while operating on connection pool: {0}")]
    PoolError(#[from] deadpool_postgres::PoolError),

    #[cfg(feature = "postgres")]
    #[error("Error occurred while building connection pool: {0}")]
    PoolBuildError(#[from] deadpool_postgres::BuildError),

    #[error("Error occurred while serializing json: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Got wrong type for column")]
    WrongType(),

    #[cfg(feature = "postgres")]
    #[error("Error receiving response from worker: {0}")]
    RecvError(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("Disconnected from database")]
    Disconnected,
}

impl<T> From<CacheError> for Result<T> {
    fn from(e: CacheError) -> Self {
        Err(e)
    }
}
