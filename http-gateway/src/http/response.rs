use crate::Error;
use serde::Serialize;
use warp::reply::Json;

#[derive(Serialize, Debug)]
pub struct ErrorResponse<'a> {
    pub error: &'a Error,
}

impl ErrorResponse<'_> {
    pub fn from(error: &Error) -> ErrorResponse {
        ErrorResponse { error }
    }
}

impl Into<warp::reply::Json> for ErrorResponse<'_> {
    fn into(self) -> Json {
        warp::reply::json(&self)
    }
}
