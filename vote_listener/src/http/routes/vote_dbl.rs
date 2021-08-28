use std::sync::Arc;

use axum::extract;

use crate::http::server::Server;

use crate::http::extractors::AuthTokenExtractor;
use crate::http::response::Response;
use axum::response::Json;
use hyper::StatusCode;
use log::{error, info};
use model::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestBody {
    #[serde(default)]
    admin: bool,
    avatar: Box<str>,
    username: Box<str>,
    id: Snowflake,
}

pub async fn vote_dbl_handler(
    auth_token: AuthTokenExtractor,
    body: extract::Json<RequestBody>,
    server: extract::Extension<Arc<Server>>,
) -> (StatusCode, Json<Response>) {
    let (server, body) = (server.0, body.0);

    if auth_token.0 != server.config.dbl_token[..] {
        return (StatusCode::UNAUTHORIZED, generate_invalid_signature());
    }

    if let Err(e) = server.database.add_vote(body.id).await {
        error!("Error while adding vote: {}", e); // TODO: Sentry
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Response::error("Database error")),
        );
    }

    info!("Logged vote for {}", body.id);

    (StatusCode::OK, Json(Response::success()))
}

fn generate_invalid_signature() -> Json<Response> {
    generate_unauthorized("Invalid signature")
}

fn generate_unauthorized(error: &str) -> Json<Response> {
    Json(Response::error(error))
}
