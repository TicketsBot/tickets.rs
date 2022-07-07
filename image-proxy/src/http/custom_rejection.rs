use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

pub struct CustomRejection(String, StatusCode);

impl From<(String, StatusCode)> for CustomRejection {
    fn from(tuple: (String, StatusCode)) -> Self {
        Self(tuple.0, tuple.1)
    }
}

impl IntoResponse for CustomRejection {
    fn into_response(self) -> Response {
        (self.1, self.0).into_response()
    }
}
