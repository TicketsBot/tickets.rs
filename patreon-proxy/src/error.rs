use thiserror::Error;

pub type Result<T> = std::result::Result<T, PatreonError>;

#[derive(Error, Debug)]
pub enum PatreonError {
    #[error("Missing tokens for client ID {0}")]
    MissingTokens(String),

    #[error("Error while performing HTTP operation: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Error while operating on JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Error while performing database operation: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),
}

impl<T> From<PatreonError> for Result<T> {
    fn from(e: PatreonError) -> Self {
        Err(e)
    }
}
