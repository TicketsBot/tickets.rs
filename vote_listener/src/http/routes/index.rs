use actix_web::{get, http, HttpResponse, Responder};

#[get("/")]
pub async fn index_handler() -> impl Responder {
    HttpResponse::Found()
        .header(http::header::LOCATION, "https://google.com")
        .finish()
        .into_body()
}