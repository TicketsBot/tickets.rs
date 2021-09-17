use thiserror::Error;

pub type Result<T> = std::result::Result<T, UpdaterError>;

#[derive(Error, Debug)]
pub enum UpdaterError {
    #[error("Error while sending HTTP request {0}")]
    ReqwestError(reqwest::Error),

    #[error("Received error response from DBL: {0}")]
    ResponseError(String),
}

impl<T> From<UpdaterError> for Result<T> {
    fn from(e: UpdaterError) -> Result<T> {
        Err(e)
    }
}
