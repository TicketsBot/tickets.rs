use hyper::StatusCode;
use serde::Serialize;
use cache::CacheError;

#[derive(Serialize, Debug)]
#[serde(untagged)]
pub enum Response<T> {
    Error { error: String },
    CacheMiss,
    Success { success: bool },
    Found(T),
}

impl<T> Response<T> {
    pub fn error(error: String) -> Response<T> {
        Response::Error {
            error,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Response::Error { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Response::CacheMiss => StatusCode::NOT_FOUND,
            Response::Found(_) | Response::Success { .. } => StatusCode::OK,
        }
    }
}

impl Response<()> {
    pub fn success() -> Response<()> {
        Response::Success { success: true }
    }
}

impl<T> Into<Response<T>> for CacheError {
    fn into(self) -> Response<T> {
        Response::error(format!("{}", self))
    }
}
