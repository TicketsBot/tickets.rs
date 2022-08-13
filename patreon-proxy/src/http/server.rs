use crate::config::Config;
use crate::patreon::Tier;

use std::sync::Arc;

use std::collections::HashMap;

use parking_lot::RwLock;
use reply::Json;
use serde::Serialize;
use serde_json::json;
use warp::http::StatusCode;
use warp::reply;
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

    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
}

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

    let count = warp::path("count")
        .and(config.clone())
        .and(data.clone())
        .and(warp::query::<HashMap<String, String>>())
        .and_then(patron_count);

    warp::serve(ping.or(is_premium).or(count)).run(addr).await;
}

async fn ping() -> Result<Json, warp::Rejection> {
    Ok(reply::json(&PingResponse { success: true }))
}

async fn is_premium(
    config: Arc<Config>,
    patrons: Arc<RwLock<HashMap<String, Tier>>>,
    query: HashMap<String, String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    if query.get("key") != Some(&config.server_key) {
        return Ok(reply::with_status(
            reply::json(&json!({
                "error": "Invalid secret key"
            })),
            StatusCode::FORBIDDEN,
        ));
    }

    let mut ids = match query.get("id") {
        Some(joined) => joined.split(','),
        None => {
            return Ok(reply::with_status(
                reply::json(&json!({
                    "error": "User ID is missing"
                })),
                StatusCode::BAD_REQUEST,
            ));
        }
    };

    let tiers = Tier::values();
    let highest_tier_id = tiers.last().unwrap().tier_id();
    let mut guild_highest_tier: Option<&Tier> = None;
    let mut user_id: Option<&str> = None;

    let patrons = patrons.read();

    // any so we stop at the first true
    // we need to find the highest tier, so we need to only break at
    ids.any(|id| {
        if let Some(tier) = patrons.get(id) {
            match guild_highest_tier {
                None => {
                    guild_highest_tier = Some(tier);
                    user_id = Some(id);
                }
                Some(current_tier) => {
                    if tier.tier_id() > current_tier.tier_id() {
                        guild_highest_tier = Some(tier);
                        user_id = Some(id);
                    }
                }
            }

            tier.tier_id() == highest_tier_id
        } else {
            false
        }
    });

    let response = PremiumResponse {
        premium: guild_highest_tier.is_some(),
        tier: guild_highest_tier.map(|tier| tier.tier_id()),
        user_id: user_id.map(|id| id.to_owned()),
    };

    Ok(reply::with_status(
        reply::json(&response),
        StatusCode::BAD_REQUEST,
    ))
}

async fn patron_count(
    config: Arc<Config>,
    patrons: Arc<RwLock<HashMap<String, Tier>>>,
    query: HashMap<String, String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    if query.get("key") != Some(&config.server_key) {
        return Ok(reply::with_status(
            reply::json(&json!({
                "error": "Invalid secret key"
            })),
            StatusCode::FORBIDDEN,
        ));
    }

    let count = patrons.read().len();

    Ok(reply::with_status(
        reply::json(&json!({
            "count": count
        })),
        StatusCode::OK,
    ))
}
