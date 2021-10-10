pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Error during HTTP request: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Error parsing UTF-8 string: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("Server returned response: {0}")]
    ResponseError(String),
}

impl<T> From<AppError> for Result<T> {
    fn from(e: AppError) -> Self {
        Err(e)
    }
}
