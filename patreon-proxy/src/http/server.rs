use crate::config::Config;
use crate::patreon::Tier;

use std::sync::Arc;
use tokio::sync::RwLock;

use std::collections::HashMap;

use serde::Serialize;
use warp::http::StatusCode;
use warp::reply::Json;
use warp::Filter;

use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Serialize, Debug)]
struct PingResponse {
    success: bool,
}

#[derive(Serialize, Debug)]
struct PremiumResponse {
    premium: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    tier: Option<i32>,
}

macro_rules! map(
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
     };
);

pub async fn listen(config: Arc<Config>, data: Arc<RwLock<HashMap<String, Tier>>>) {
    let addr = SocketAddr::from_str(&config.server_addr).unwrap();

    let ping = warp::path("ping").and_then(ping);

    let config = warp::any().map(move || Arc::clone(&config));
    let data = warp::any().map(move || Arc::clone(&data));

    let is_premium = warp::path("ispremium")
        .and(config.clone())
        .and(data.clone())
        .and(warp::query::<HashMap<String, String>>())
        .and_then(is_premium);

    warp::serve(ping.or(is_premium)).run(addr).await;
}

async fn ping() -> Result<Json, warp::Rejection> {
    Ok(warp::reply::json(&PingResponse { success: true }))
}

async fn is_premium(
    config: Arc<Config>,
    data: Arc<RwLock<HashMap<String, Tier>>>,
    query: HashMap<String, String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    if query.get("key") != Some(&config.server_key) {
        return Ok(warp::reply::with_status(
            warp::reply::json(&map! { "error" => "Invalid secret key" }),
            StatusCode::FORBIDDEN,
        ));
    }

    let id = query.get("id");
    if id.is_none() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&map! { "error" => "User ID is missing" }),
            StatusCode::BAD_REQUEST,
        ));
    }

    let data = data.read().await;
    let tier = data.get(id.unwrap()); // safe to unwrap, we've checked if it's None

    let response = PremiumResponse {
        premium: tier.is_some(),
        tier: tier.map(|tier| tier.tier_id()),
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&response),
        StatusCode::BAD_REQUEST,
    ))
}
