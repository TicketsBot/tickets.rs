use super::CustomRejection;
use super::Server;
use crate::Config;
use axum::body::{Bytes, Full};
use axum::http::StatusCode;
use axum::response::Response;
use axum::{extract::Json, Extension};
use hyper::body::HttpBody;
#[cfg(feature = "pre-resolve")]
use hyper::client::connect::dns::Name;
use hyper::{Method, Request};
use log::{debug, warn};
use serde::Deserialize;
use std::collections::HashMap;
#[cfg(feature = "pre-resolve")]
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
#[cfg(feature = "pre-resolve")]
use tower::Service;
use url::form_urlencoded::Serializer;
#[cfg(feature = "pre-resolve")]
use url::Host;
use url::Url;

// Max 32Kb of JSON
const MAX_RESPONSE_SIZE: u64 = 64 * 1024;

#[derive(Debug, Deserialize)]
pub(crate) struct Payload {
    pub url: String,
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,

    #[serde(default)]
    pub json_body: Option<serde_json::Value>, // JSON body: takes precedence over body
    #[serde(default)]
    pub body: Option<String>, // base64 encoded
}

pub(crate) async fn proxy(
    Json(mut data): Json<Payload>,
    Extension(server): Extension<Arc<Server>>,
) -> Result<Response<Full<Bytes>>, CustomRejection> {
    #[cfg(feature = "pre-resolve")]
    {
        let url = Url::parse(data.url.as_str()).unwrap();

        let is_private_ip = match url.host() {
            Some(Host::Ipv4(addr)) => !addr.is_global(),
            Some(Host::Ipv6(addr)) => !addr.is_global(),
            Some(Host::Domain(domain)) => {
                let name = match Name::from_str(domain) {
                    Ok(name) => name,
                    Err(_) => return Err((StatusCode::BAD_REQUEST, "Invalid domain name").into()),
                };

                // Internally arc-ed, we can safely clone
                let mut resolved = match server.resolver.clone().call(name).await {
                    Ok(resolved) => resolved,
                    Err(_) => {
                        return Err((StatusCode::BAD_REQUEST, "Failed to resolve hostname").into())
                    }
                };

                resolved.any(|addr| !addr.ip().is_global())
            }
            _ => false,
        };

        if is_private_ip {
            return Err((StatusCode::FORBIDDEN, "Cannot proxy request to private IP").into());
        }
    }

    let method: Method = match data.method.to_uppercase().as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        _ => return Err((StatusCode::METHOD_NOT_ALLOWED, "Unsupported method").into()),
    };

    let url_params = Serializer::new(String::new())
        .append_pair("url", data.url.as_str())
        .finish();

    let mut proxy_url = Url::parse(server.config.worker_url.as_str()).unwrap();
    proxy_url.set_query(Some(url_params.as_str()));

    let mut req = Request::builder()
        .method(method)
        .uri(proxy_url.as_str())
        .header(
            server.config.auth_header_name.as_str(),
            server.config.auth_header_value.as_str(),
        );

    // Add client-requested headers
    filter_headers(&server.config, &mut data.headers);
    for (key, value) in data.headers {
        req = req.header(key.as_str(), value.as_str());
    }

    let body = if let Some(body) = data.json_body {
        req = req.header(hyper::header::CONTENT_TYPE, "application/json");

        let Ok(json) = serde_json::to_string(&body) else {
            //return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to serialize JSON").into());
            return Err((StatusCode::METHOD_NOT_ALLOWED, "Unsupported method").into());
        };

        json.into()
    } else if let Some(body) = data.body {
        let Ok(decoded) = base64::decode(body.as_str()) else {
            return Err((StatusCode::BAD_REQUEST, "Invalid base64 body").into());
        };

        decoded.into()
    } else {
        hyper::Body::empty()
    };

    let Ok(req) = req.body(body) else {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Failed to set body").into());
    };

    let future = server.http_client.request(req);

    let res = match timeout(Duration::from_secs(3), future).await {
        Ok(res) => res,
        Err(_) => return Err((StatusCode::REQUEST_TIMEOUT, "Request timed out").into()),
    };

    let res = match res {
        Ok(res) => res,
        Err(e) => {
            debug!("Failed to proxy request to {}: {e}", data.url);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Request failed").into());
        }
    };

    if let Some(proxy_error) = res
        .headers()
        .get("x-proxy-error")
        .and_then(|header| header.to_str().ok())
    {
        warn!("Proxy error: {}", proxy_error);
        return Err((StatusCode::INTERNAL_SERVER_ERROR, "Request failed").into());
    }

    // Enforce max size limit
    if let Some(content_length) = res.size_hint().upper() {
        debug!(
            "length {content_length} max: {MAX_RESPONSE_SIZE}. {:?}",
            res.body().size_hint()
        );

        if content_length > MAX_RESPONSE_SIZE {
            return Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                "Response exceeded size limit",
            )
                .into());
        }
    }

    let proxied_response = Response::builder()
        .status(StatusCode::OK)
        .header("x-status-code", res.status().as_u16());

    // Read data
    let data = match hyper::body::to_bytes(res.into_body()).await {
        Ok(data) => data,
        Err(e) => {
            warn!("Failed to read response from {}: {e}", data.url);
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Error reading data").into());
        }
    };

    let proxied_response = match proxied_response.body(Full::from(data)) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to set response body: {e}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error setting response body",
            )
                .into());
        }
    };

    Ok(proxied_response)
}

static BLACKLISTED_HEADERS: &[&str] = &[
    "user-agent",
    "x-real-ip",
    "cache-control",
    "content-type",
    "content-length",
    "expect",
    "max-forwards",
    "pragma",
    "range",
    "te",
    "if-match",
    "if-none-match",
    "if-modified-since",
    "if-unmodified-since",
    "if-range",
    "accept",
    "from",
    "referer",
];
static BLACKLISTED_HEADER_PREFIXES: &[&str] = &["cf-", "x-proxy-", "x-forwarded"];

fn filter_headers(config: &Config, headers: &mut HashMap<String, String>) {
    headers.retain(|name, _| {
        let name = name.to_lowercase();

        !BLACKLISTED_HEADERS.contains(&name.as_str())
            && config.auth_header_name.as_str() != name.as_str()
            && !BLACKLISTED_HEADER_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
    });
}
