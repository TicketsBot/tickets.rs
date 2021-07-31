use axum::{async_trait, extract::{FromRequest, RequestParts}};
use hyper::http::{StatusCode, header};
use axum::response::Json;
use crate::http::response::Response;

pub struct AuthTokenExtractor(pub String);

#[async_trait]
impl<B> FromRequest<B> for AuthTokenExtractor
    where
        B: Send,
{
    type Rejection = (StatusCode, Json<Response>);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let token = req.headers()
            .and_then(|headers| headers.get(header::AUTHORIZATION))
            .and_then(|header| header.to_str().ok());

        if let Some(token) = token {
            Ok(Self(token.to_owned()))
        } else {
            Err((StatusCode::BAD_REQUEST, Json(Response::error("Missing Authorization header"))))
        }
    }
}