use thiserror::Error;

#[derive(Error, Debug)]
pub enum UpdaterError {
    #[error("Error while sending HTTP request {0}")]
    ReqwestError(reqwest::Error),

    #[error("Received error response from DBL: {0}")]
    ResponseError(String),
}

impl<T> Into<Result<T, UpdaterError>> for UpdaterError {
    fn into(self) -> Result<T, UpdaterError> {
        Err(self)
    }
}