use crate::http::Server;
use axum::extract;
use axum::http::Uri;
use axum::response::Redirect;
use std::str::FromStr;
use std::sync::Arc;

pub async fn index_handler(server: extract::Extension<Arc<Server>>) -> Redirect {
    let uri = Uri::from_str(&server.0.config.vote_url[..]).expect("Invalid redirect URI");
    Redirect::found(uri)
}
