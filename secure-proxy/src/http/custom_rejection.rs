use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use hyper::header::HeaderName;
use hyper::http::HeaderValue;

pub struct CustomRejection(StatusCode, String);

impl From<(StatusCode, &str)> for CustomRejection {
    fn from(tuple: (StatusCode, &str)) -> Self {
        Self(tuple.0, tuple.1.to_owned())
    }
}

impl IntoResponse for CustomRejection {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-proxy-error"),
            HeaderValue::from_str(self.1.as_str()).unwrap(),
        );

        (self.0, headers).into_response()
    }
}
