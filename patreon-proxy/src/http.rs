use crate::patreon::Tier;

use std::sync::{Arc, RwLock};

use std::collections::HashMap;

use reply::Json;
use serde::Serialize;
use serde_json::json;
use warp::http::StatusCode;
use warp::reply;
use warp::Filter;

use std::net::SocketAddr;
use std::str::FromStr;
use tracing::log::error;

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

pub async fn listen(server_addr: &str, data: Arc<RwLock<HashMap<String, Tier>>>) {
    let addr = SocketAddr::from_str(server_addr).unwrap();

    let ping = warp::path("ping").and_then(ping);

    let data = warp::any().map(move || Arc::clone(&data));

    let is_premium = warp::path("ispremium")
        .and(data.clone())
        .and(warp::query::<HashMap<String, String>>())
        .and_then(is_premium);

    let all = warp::path("all").and(data.clone()).and_then(all_patrons);
    let count = warp::path("count").and(data.clone()).and_then(patron_count);

    warp::serve(ping.or(is_premium).or(all).or(count)).run(addr).await;
}

async fn ping() -> Result<Json, warp::Rejection> {
    Ok(reply::json(&PingResponse { success: true }))
}

async fn is_premium(
    patrons: Arc<RwLock<HashMap<String, Tier>>>,
    query: HashMap<String, String>,
) -> Result<impl warp::Reply, warp::Rejection> {
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

    let patrons = patrons.read().unwrap();

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
    patrons: Arc<RwLock<HashMap<String, Tier>>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let count = patrons.read().unwrap().len();

    Ok(reply::with_status(
        reply::json(&json!({
            "count": count
        })),
        StatusCode::OK,
    ))
}

async fn all_patrons(
    patrons: Arc<RwLock<HashMap<String, Tier>>>
) -> Result<impl warp::Reply, warp::Rejection> {
    let patrons = patrons.read().unwrap();

    let value = match serde_json::to_value(&*patrons) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to serialize patrons: {}", e);

            return Ok(reply::with_status(
                reply::json(&json!({
                    "error": "Unable to serialize patrons map"
                })),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        },
    };

    Ok(reply::with_status(
        reply::json(&json!(value)),
        StatusCode::OK,
    ))
}
