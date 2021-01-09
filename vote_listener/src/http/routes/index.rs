use actix_web::{get, http, HttpResponse, Responder, web::Data};
use std::sync::Arc;
use crate::http::Server;

#[get("/")]
pub async fn index_handler(server: Data<Arc<Server>>) -> impl Responder {
    HttpResponse::Found()
        .header(http::header::LOCATION, &*server.config.vote_url)
        .finish()
        .into_body()
}