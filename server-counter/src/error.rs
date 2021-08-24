use std::net;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("error occurred during parsing address: {0}")]
    AddrParseError(#[from] net::AddrParseError),

    #[error("error occurred during cache operation: {0}")]
    CacheError(#[from] cache::CacheError),

    #[error("error occurred in hyper: {0}")]
    HyperError(#[from] hyper::Error),
}
