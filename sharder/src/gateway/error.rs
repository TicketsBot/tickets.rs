use thiserror::Error;
use std::fmt::Display;
use crate::gateway::outbound_message::OutboundMessage;
use crate::manager::FatalError;

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

    #[error("error while operating on websocket: {0}")]
    WebsocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("error while reading oneshot channel: {0}")]
    RecvError(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("error while sending message to writer: {0}")]
    SendMessageError(#[from] tokio::sync::mpsc::error::SendError<OutboundMessage>),

    #[error("error while sending message to error chan: {0}")]
    SendErrorError(#[from] tokio::sync::mpsc::error::SendError<FatalError>),
}

impl GatewayError {
    pub fn custom<T: Display>(msg: T) -> GatewayError {
        GatewayError::GenericError(msg.to_string())
    }
}