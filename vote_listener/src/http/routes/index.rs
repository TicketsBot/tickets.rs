use crate::http::Server;
use axum::body::Body;
use axum::prelude::*;
use hyper::{http, Response};
use std::sync::Arc;

pub async fn index_handler(server: extract::Extension<Arc<Server>>) -> Response<Body> {
    Response::builder()
        .status(http::status::StatusCode::FOUND)
        .header(http::header::LOCATION, &server.0.config.vote_url[..])
        .body(Body::empty())
        .expect("Failed to build body") // Should be impossible
}
