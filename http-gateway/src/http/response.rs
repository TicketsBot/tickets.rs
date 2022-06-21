use crate::Error;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct ErrorResponse<'a> {
    pub error: &'a Error,
}

impl<'a> From<&'a Error> for ErrorResponse<'a> {
    fn from(e: &'a Error) -> Self {
        Self { error: e }
    }
}

impl From<ErrorResponse<'_>> for warp::reply::Json {
    fn from(e: ErrorResponse<'_>) -> Self {
        warp::reply::json(&e)
    }
}
