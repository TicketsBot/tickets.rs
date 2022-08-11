use crate::http::response::Response;
use crate::http::Server;
use axum::extract::Extension;
use axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN;
use axum::http::{HeaderMap, HeaderValue};
use axum::response::Json;
use cache::Cache;
use hyper::http::StatusCode;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub async fn total_handler<T: Cache>(
    server: Extension<Arc<Server<T>>>,
) -> (StatusCode, HeaderMap, Json<Response>) {
    let count = server.0.count.load(Ordering::Relaxed);

    let mut headers = HeaderMap::new();
    headers.insert(
        ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("https://ticketsbot.net"),
    );

    let body = Response::success(count);

    (StatusCode::OK, headers, Json(body))
}
