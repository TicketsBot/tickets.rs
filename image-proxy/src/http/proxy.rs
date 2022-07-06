use crate::http::Server;
use axum::extract::Query;
use axum::http::{HeaderMap, StatusCode};
use axum::Extension;
use jwt::VerifyWithKey;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;
use axum::response::IntoResponse;
use url::{Host, Url};

pub(crate) async fn proxy(
    Query(params): Query<HashMap<String, String>>,
    Extension(server): Extension<Arc<Server>>,
) -> Result<impl IntoResponse, StatusCode> {
    let token = params.get("token").ok_or(StatusCode::UNAUTHORIZED)?;

    let claims: BTreeMap<String, String> = token
        .verify_with_key(&server.jwt_key)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    println!("{:?}", claims);
    let raw_url = claims.get("url").ok_or(StatusCode::BAD_REQUEST)?;

    // Verify URL is valid, and not a private IP range
    let url = Url::parse(raw_url).map_err(|_| StatusCode::BAD_REQUEST)?;
    match url.host() {
        Some(Host::Ipv4(addr)) => {

        }
    }
    let host = url.host().unwrap();

    let res = server.http_client.get(url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Check image is not larger than 8MB
    if let Some(content_length) = res.content_length() {
        if content_length > 8 * 1024 * 1024 {
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
    }

    // Verify response is an image
    let content_type = res.headers().get("Content-Type")
        .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?
        .to_str()
        .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    if !content_type.to_lowercase().starts_with("image/") {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let mut response_headers = HeaderMap::new();
    response_headers.insert("content-type", res.headers().get("Content-Type").unwrap().clone()); // Impossible to fail

    // Read data
    let data = res.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok((response_headers, data))
}
