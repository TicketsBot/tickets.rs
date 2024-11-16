use model::Snowflake;
use serde::Serializer;
use std::fmt::Debug;
use warp::reject::Reject;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid ed25519 signature length")]
    InvalidSignatureLength,

    #[error("invalid ed25519 signature: {0}")]
    InvalidSignatureFormat(#[from] hex::FromHexError),

    #[error("invalid ed25519 signature: {0}")]
    InvalidSignature(#[from] ed25519_dalek::SignatureError),

    #[error("application_id does not match. got {0}, expected {1}")]
    InvalidApplicationId(Snowflake, Snowflake),

    #[error("error while decoding json payload: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("error while decoding string: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("error while performing database operation: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("error while performing cache operation: {0}")]
    CacheError(#[from] cache::CacheError),

    #[error("bot with ID {0} not found")]
    BotNotFound(Snowflake),

    #[error("error occurred while forwarding event to worker: {0}")]
    ReqwestError(reqwest::Error),

    #[error("guild_id was missing from request")]
    MissingGuildId,

    #[error("interaction type is unsupported")]
    UnsupportedInteractionType,
}

impl Reject for Error {}

impl<T> From<Error> for Result<T, Error> {
    fn from(e: Error) -> Self {
        Err(e)
    }
}

impl serde::Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{}", self)[..])
    }
}
