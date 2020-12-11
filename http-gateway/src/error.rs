use warp::reject::Reject;
use warp::Rejection;
use serde::Serializer;
use std::fmt::Debug;
use deadpool_redis::redis;
use model::Snowflake;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("invalid ed25519 signature length")]
    InvalidSignatureLength,

    #[error("invalid ed25519 signature: {0}")]
    InvalidSignatureFormat(#[from] hex::FromHexError),

    #[error("invalid ed25519 signature: {0}")]
    InvalidSignature(#[from] ed25519_dalek::SignatureError),

    #[error("error while decoding json payload: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("error while decoding string: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("error while performing database operation: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("error while getting redis conn: {0}")]
    PoolError(#[from] deadpool::managed::PoolError<redis::RedisError>),

    #[error("error while operating on Redis: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("token not found for bot {0}")]
    TokenNotFound(Snowflake),
}

impl Reject for Error {}

impl Into<warp::Rejection> for Error {
    fn into(self) -> Rejection {
        warp::reject::custom(self)
    }
}

impl<T> Into<Result<T, Self>> for Error {
    fn into(self) -> Result<T, Self> {
        Err(self)
    }
}

impl serde::Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}", self)[..])
    }
}