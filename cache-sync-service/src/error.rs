pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("envy error: {0}")]
    EnvyError(#[from] envy::Error),

    #[error("serde_json error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("cache error: {0}")]
    CacheError(#[from] cache::CacheError),

    #[error("event stream error: {0}")]
    StreamError(#[from] event_stream::StreamError),
}
