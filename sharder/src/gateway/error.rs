use crate::gateway::outbound_message::OutboundMessage;
use crate::CloseEvent;
use std::fmt::Display;
use thiserror::Error;

pub type Result<T, E = GatewayError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("t value on dispatch was not a string")]
    MissingEventType,

    #[error("d value on dispatch was not an object")]
    MissingEventData,

    #[error("shard had a None bot_id field")]
    NoneId,

    #[error("{0}")]
    GenericError(String),

    #[error("error while operating on json (serde): {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("error while operating on Redis: {0}")]
    RedisError(#[from] deadpool_redis::redis::RedisError),

    #[error("error while getting redis conn: {0}")]
    PoolError(#[from] deadpool::managed::PoolError<deadpool_redis::redis::RedisError>),

    #[error("error while operating on websocket: {0}")]
    WebsocketError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("error while writing to websocket: {0}")]
    WebsocketSendError(#[from] futures::channel::mpsc::SendError),

    #[error("error while reading oneshot channel: {0}")]
    RecvError(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("oneshot receiver already hung up")]
    ReceiverHungUpError,

    #[error("error while sending message to writer: {0}")]
    SendMessageError(#[from] tokio::sync::mpsc::error::SendError<OutboundMessage>),

    #[error("error while sending message to chan: {0}")]
    SendError(#[from] tokio::sync::mpsc::error::SendError<()>),

    #[error("error while sending message to chan: {0}")]
    SendU16Error(#[from] tokio::sync::mpsc::error::SendError<u16>),

    #[cfg(feature = "compression")]
    #[error("error occurred while compressing payload: {0}")]
    CompressError(#[from] flate2::CompressError),

    #[cfg(feature = "compression")]
    #[error("error occurred while decompressing payload: {0}")]
    DecompressError(#[from] flate2::DecompressError),

    #[error("error occurred while operating on the cache: {0}")]
    CacheError(#[from] cache::CacheError),

    #[error("error occurred while operating on database: {0}")]
    DatabaseError(#[from] database::sqlx::Error),

    #[error("bot ID was missing on whitelabel identify")]
    MissingBotId,

    #[error("json was missing field {0}")]
    MissingFieldError(String),

    #[error("error occurred while parsing utf8 bytes: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("[{:?}] {:?}", .data.status_code, .data.error)]
    AuthenticationError { bot_token: String, data: CloseEvent },

    #[error("error occurred while forwarding event to worker over HTTP: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Received error response from worker: {0}")]
    WorkerError(String),

    #[error("error occurred while parsing URL: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("redis returned wrong result type")]
    WrongResultType,

    #[error("redis returned wrong result length: expected {expected}, got {actual}")]
    WrongResultLength { expected: usize, actual: usize },

    #[error("error occurred while parsing int: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("error occurred while performing I/O operation: {0}")]
    IoError(#[from] std::io::Error),

    #[cfg(feature = "metrics")]
    #[error("error occurred while parsing socket address: {0}")]
    AddrParseError(#[from] std::net::AddrParseError),

    #[cfg(feature = "metrics")]
    #[error("error occurred while running hyper server: {0}")]
    HyperError(#[from] hyper::Error),

    #[error("error occurred while streaming events: {0}")]
    StreamError(#[from] event_stream::StreamError),
}

impl GatewayError {
    pub fn custom(msg: impl Display) -> Self {
        GatewayError::GenericError(msg.to_string())
    }
}

impl<T> From<GatewayError> for Result<T> {
    fn from(e: GatewayError) -> Self {
        Err(e)
    }
}
