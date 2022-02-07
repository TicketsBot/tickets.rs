#[cfg(feature = "postgres")]
use crate::CachePayload;
use model::Snowflake;

pub type Result<T> = std::result::Result<T, CacheError>;

#[derive(thiserror::Error, Debug)]
pub enum CacheError {
    #[cfg(feature = "postgres")]
    #[error("Error occurred while interacting with DB: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),

    #[error("Error occurred while serializing json: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Got wrong type for column")]
    WrongType(),

    #[cfg(feature = "postgres")]
    #[error("Error sending cache payload to worker: {0}")]
    SendError(#[from] tokio::sync::mpsc::error::SendError<CachePayload>),

    #[cfg(feature = "postgres")]
    #[error("Error receiving response from worker: {0}")]
    RecvError(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("Disconnected from database")]
    Disconnected,

    #[cfg(feature = "cache-model")]
    #[error("Guild with ID {0} not found")]
    GuildNotFound(Snowflake),

    #[cfg(feature = "memory")]
    #[error("Tried to insert a Member object into cache, was missing User object")]
    MemberMissingUser,

    #[cfg(feature = "memory")]
    #[error("Requested an entity from cache, but the store for the entity type is disabled")]
    StoreDisabled,

    #[cfg(feature = "cache-model")]
    #[error("Struct {0} was missing a field: {1}")]
    MissingField(String, String),

    #[cfg(feature = "client")]
    #[error("Error during HTTP request: {0}")]
    ReqwestError(reqwest::Error),

    #[cfg(feature = "client")]
    #[error("Error during HTTP request: {0}")]
    ResponseError(String),
}

impl<T> From<CacheError> for Result<T> {
    fn from(e: CacheError) -> Self {
        Err(e)
    }
}
