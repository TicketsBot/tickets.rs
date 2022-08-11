use crate::http::Server;
use axum::extract::Extension;
use cache::Cache;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub async fn prometheus_handler<T: Cache>(server: Extension<Arc<Server<T>>>) -> String {
    let count = server.0.count.load(Ordering::Relaxed);
    format!("tickets_servercount {}", count)
}
