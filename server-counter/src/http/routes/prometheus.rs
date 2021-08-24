use crate::http::Server;
use axum::extract;
use cache::Cache;
use std::sync::Arc;

pub async fn prometheus_handler<T: Cache>(server: extract::Extension<Arc<Server<T>>>) -> String {
    let count = *server.0.count.read();
    format!("tickets_servercount {}", count)
}
