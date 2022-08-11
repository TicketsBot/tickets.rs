use crate::http::Server;
use axum::extract;
use axum::response::Redirect;
use std::sync::Arc;

pub async fn index_handler(server: extract::Extension<Arc<Server>>) -> Redirect {
    Redirect::permanent(server.0.config.vote_url.as_str())
}
