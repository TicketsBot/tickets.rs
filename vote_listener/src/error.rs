use bb8_postgres::tokio_postgres;
use std::{io, net};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("error occurred during parsing address: {0}")]
    AddrParseError(#[from] net::AddrParseError),

    #[error("error occurred during I/O operation: {0}")]
    IoError(#[from] io::Error),

    #[error("error occurred during database operation: {0}")]
    DatabaseError(#[from] tokio_postgres::Error),

    #[error("error occurred during database pool operation: {0}")]
    PoolError(#[from] bb8::RunError<tokio_postgres::Error>),

    #[error("error occurred during JSON operation: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("error occurred in hyper: {0}")]
    HyperError(#[from] hyper::Error),
}
