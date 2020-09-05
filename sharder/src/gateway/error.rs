use thiserror::Error;
use std::fmt::Display;

#[derive(Error, Debug)]
pub enum GatewayError {
    /*#[error("invalid opcode {0}")]
    InvalidOpcode(u8),*/

    #[error("t value on dispatch was not a string: {0}")]
    TypeNotString(serde_json::Value),

    #[error("shard had a None bot_id field")]
    NoneId,

    #[error("{0}")]
    GenericError(String),

    #[error("error while encoding redis payload: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("error while obtaining connection from pool: {0}")]
    PoolError(#[from] r2d2::Error),

    #[error("error while operating on Redis: {0}")]
    RedisError(#[from] r2d2_redis::redis::RedisError),
}

impl GatewayError {
    pub fn custom<T: Display>(msg: T) -> GatewayError {
        GatewayError::GenericError(msg.to_string())
    }
}