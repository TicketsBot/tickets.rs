use crate::Error;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct ErrorResponse<'a> {
    pub error: &'a Error,
}

impl ErrorResponse<'_> {
    pub fn from(error: &Error) -> ErrorResponse {
        ErrorResponse { error }
    }
}

impl From<ErrorResponse<'_>> for warp::reply::Json {
    fn from(e: ErrorResponse<'_>) -> Self {
        warp::reply::json(&e)
    }
}
