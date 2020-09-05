use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Error occurred while interacting with DB: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Error occurred while serializing json: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Got wrong type for column")]
    WrongType(),
}
