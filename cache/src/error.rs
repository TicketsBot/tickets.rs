use crate::CachePayload;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Error occurred while interacting with DB: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),

    #[error("Error occurred while serializing json: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Got wrong type for column")]
    WrongType(),

    #[error("Error sending cache payload to worker: {0}")]
    SendError(#[from] tokio::sync::mpsc::error::SendError<CachePayload>),

    #[error("Error receiving response from worker: {0}")]
    RecvError(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("Disconnected from database")]
    Disconnected,
}

impl<T> Into<Result<T, Self>> for CacheError {
    fn into(self) -> Result<T, Self> {
        Err(self)
    }
}
