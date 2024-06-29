use crate::http::{CustomRejection, Server};
use axum::extract::Query;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Extension;
use hyper::body::HttpBody;
use hyper::Uri;
use jwt::VerifyWithKey;
use log::{debug, info, warn};
use std::collections::{BTreeMap, HashMap};
use std::ops::Add;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::time::{timeout, Instant};
use url::{Host, Url};

const MAX_RESPONSE_SIZE: u64 = 8 * 1024 * 1024;

pub(crate) async fn proxy(
    Query(params): Query<HashMap<String, String>>,
    Extension(server): Extension<Arc<Server>>,
) -> Result<impl IntoResponse, CustomRejection> {
    let token = params
        .get("token")
        .ok_or(("Missing token".to_string(), StatusCode::UNAUTHORIZED))?;

    let claims: BTreeMap<String, String> = token
        .verify_with_key(&server.jwt_key)
        .map_err(|_| ("Invalid token".to_string(), StatusCode::UNAUTHORIZED))?;

    // Extract request URL
    let raw_url = claims
        .get("url")
        .ok_or(("Missing URL".to_string(), StatusCode::BAD_REQUEST))?;
    info!("Proxying request to {raw_url}");

    // Extract request expire time
    let expire_unix_seconds = claims
        .get("exp")
        .ok_or(("Missing expiry".to_string(), StatusCode::BAD_REQUEST))?
        .parse::<u64>()
        .map_err(|_| ("Invalid expiry".to_string(), StatusCode::BAD_REQUEST))?;

    let time_now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Failed to get unix seconds")
        .as_secs();

    if time_now > expire_unix_seconds {
        return Err(("Token expired".to_string(), StatusCode::UNAUTHORIZED).into());
    }

    let expires_at = Instant::now().add(Duration::from_secs(expire_unix_seconds - time_now));

    // Extract unique request ID
    let request_id = claims
        .get("request_id")
        .ok_or(("Missing request ID".to_string(), StatusCode::BAD_REQUEST))?
        .parse::<uuid::Uuid>()
        .map_err(|_| ("Invalid request ID".to_string(), StatusCode::BAD_REQUEST))?;

    // Check that request ID is not already used
    let is_fresh = server.used_token_store.add(request_id, expires_at);
    if !is_fresh {
        return Err(("Token already used".to_string(), StatusCode::BAD_REQUEST).into());
    }

    // Verify URL is valid, and not a private IP range
    let url =
        Url::parse(raw_url).map_err(|_| ("Invalid URL".to_string(), StatusCode::BAD_REQUEST))?;
    let is_private_ip = match url.host() {
        Some(Host::Ipv4(addr)) => !addr.is_global(),
        Some(Host::Ipv6(addr)) => !addr.is_global(),
        Some(Host::Domain(_)) => false, // Handled by GlobalResolver
        _ => false,
    };

    if is_private_ip {
        info!("Request to {raw_url} was to private IP range");
        return Err(("Invalid URL".to_string(), StatusCode::BAD_REQUEST).into());
    }

    // Shouldn't fail
    let uri =
        Uri::from_str(raw_url).map_err(|_| ("Invalid URI".to_string(), StatusCode::BAD_REQUEST))?;

    let fut = server.http_client.get(uri);

    let res = match timeout(Duration::from_secs(3), fut).await {
        Ok(res) => res,
        Err(_) => {
            return Err(("Request timed out".to_string(), StatusCode::REQUEST_TIMEOUT).into())
        }
    };

    let res = match res {
        Ok(res) => res,
        Err(e) => {
            debug!("Failed to proxy request to {raw_url}: {e}");
            return Err((
                "Request failed".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
                .into());
        }
    };

    // Check image is not larger than 8MB
    if res
        .body()
        .size_hint()
        .upper()
        .unwrap_or(MAX_RESPONSE_SIZE + 1)
        > MAX_RESPONSE_SIZE
    {
        debug!("Request to {raw_url} was too large");
        return Err((
            "Image is over 8MB".to_string(),
            StatusCode::PAYLOAD_TOO_LARGE,
        )
            .into());
    }

    // Verify response is an image
    let content_type = res
        .headers()
        .get("Content-Type")
        .ok_or((
            "Missing content-type header".to_string(),
            StatusCode::UNPROCESSABLE_ENTITY,
        ))?
        .to_str()
        .map_err(|_| {
            (
                "Invalid content-type header".to_string(),
                StatusCode::UNPROCESSABLE_ENTITY,
            )
        })?;

    if !content_type.to_lowercase().starts_with("image/") {
        debug!("Request to {raw_url} returned non-image content-type ({content_type})");
        return Err((
            "URL did not return an image".to_string(),
            StatusCode::UNPROCESSABLE_ENTITY,
        )
            .into());
    }

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        "content-type",
        res.headers().get("Content-Type").unwrap().clone(),
    ); // Impossible to fail

    // Read data
    let data = match hyper::body::to_bytes(res.into_body()).await {
        Ok(data) => data,
        Err(e) => {
            warn!("Failed to read response from {raw_url}: {e}");
            return Err((
                "Error reading data".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            )
                .into());
        }
    };

    Ok((response_headers, data))
}
