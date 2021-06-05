use std::sync::Arc;

use actix_web::{http::HeaderValue, post, web::Data, web::Json, HttpRequest, HttpResponse};

use crate::http::response::ErrorResponse;
use crate::http::server::Server;

use model::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Body {
    admin: bool,
    avatar: Box<str>,
    username: Box<str>,
    id: Snowflake,
}

#[post("/vote/dbl")]
pub async fn vote_dbl_handler(
    req: HttpRequest,
    body: Json<Body>,
    server: Data<Arc<Server>>,
) -> HttpResponse {
    match req.headers().get("Authorization").map(&HeaderValue::to_str) {
        Some(Ok(header)) => {
            if header != &*server.config.dbl_signature {
                return generate_invalid_signature();
            }
        }
        Some(Err(e)) => {
            return generate_unauthorized(&e.to_string()[..]);
        }
        None => {
            return generate_invalid_signature();
        }
    }

    if let Err(e) = server.database.add_vote(body.id).await {
        eprintln!("Error while adding vote: {}", e); // TODO: Sentry
        return HttpResponse::InternalServerError()
            .json(ErrorResponse::new(Box::from(format!("{}", e))))
            .into_body();
    }

    HttpResponse::Ok().finish().into_body()
}

fn generate_invalid_signature() -> HttpResponse {
    generate_unauthorized("Invalid signature")
}

fn generate_unauthorized(error: &str) -> HttpResponse {
    HttpResponse::Unauthorized()
        .json(ErrorResponse::new(Box::from(error)))
        .into_body()
}
