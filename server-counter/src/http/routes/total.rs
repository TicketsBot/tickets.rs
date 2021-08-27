use crate::http::response::Response;
use crate::http::Server;
use axum::extract;
use axum::response::Json;
use cache::Cache;
use hyper::http::StatusCode;
use std::sync::Arc;
use axum::http::{HeaderMap, HeaderValue};
use axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN;

pub async fn total_handler<T: Cache>(
    server: extract::Extension<Arc<Server<T>>>,
) -> (StatusCode, HeaderMap, Json<Response>) {
    let count = *server.0.count.read();

    let mut headers = HeaderMap::new();
    headers.insert(
        ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("https://ticketsbot.net"),
    );

    let body = Response::success(count);

    (StatusCode::OK, headers, Json(body))
}
