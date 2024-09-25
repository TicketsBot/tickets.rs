pub type Result<T> = std::result::Result<T, StreamError>;

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("Kafka error: {0}")]
    KafkaError(#[from] rdkafka::error::KafkaError),

    #[error("serde_json error: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl<T> From<StreamError> for Result<T> {
    fn from(e: StreamError) -> Self {
        Err(e)
    }
}
