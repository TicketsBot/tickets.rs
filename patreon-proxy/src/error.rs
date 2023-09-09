use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Missing tokens for client ID {0}")]
    MissingTokens(String),

    #[error("Error while performing HTTP operation: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Error while operating on JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Error while performing database operation: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),

    #[error("Error while parsing URL: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("Error while creating database pool: {0}")]
    CreatePoolError(#[from] deadpool_postgres::CreatePoolError),

    #[error("{0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("Error while managing database pool: {0}")]
    PoolError(#[from] deadpool_postgres::PoolError),
}

impl<T> From<Error> for Result<T> {
    fn from(e: Error) -> Self {
        Err(e)
    }
}
