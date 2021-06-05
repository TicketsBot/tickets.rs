use crate::http::Server;
use actix_web::{get, http, web::Data, HttpResponse, Responder};
use std::sync::Arc;

#[get("/")]
pub async fn index_handler(server: Data<Arc<Server>>) -> impl Responder {
    HttpResponse::Found()
        .header(http::header::LOCATION, &*server.config.vote_url)
        .finish()
        .into_body()
}
