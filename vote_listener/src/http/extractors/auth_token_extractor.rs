use crate::http::response::Response;
use axum::response::Json;
use axum::{
    async_trait,
    extract::{FromRequest, RequestParts},
};
use hyper::http::{header, StatusCode};

pub struct AuthTokenExtractor(pub String);

#[async_trait]
impl<B> FromRequest<B> for AuthTokenExtractor
where
    B: Send,
{
    type Rejection = (StatusCode, Json<Response>);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let token = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        if let Some(token) = token {
            Ok(Self(token.to_owned()))
        } else {
            Err((
                StatusCode::BAD_REQUEST,
                Json(Response::error("Missing Authorization header")),
            ))
        }
    }
}
