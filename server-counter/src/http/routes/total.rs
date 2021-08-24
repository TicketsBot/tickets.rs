use crate::http::response::Response;
use crate::http::Server;
use axum::extract;
use axum::response::Json;
use cache::Cache;
use hyper::http::StatusCode;
use std::sync::Arc;

pub async fn total_handler<T: Cache>(
    server: extract::Extension<Arc<Server<T>>>,
) -> (StatusCode, Json<Response>) {
    let count = *server.0.count.read();
    (StatusCode::OK, Json(Response::success(count)))
}
