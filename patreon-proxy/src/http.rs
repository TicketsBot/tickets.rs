use crate::patreon::{Entitlement, Tier};

use std::sync::{Arc, RwLock};

use std::collections::HashMap;

use chrono::{DateTime, Utc};
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
    tier: i32,
    user_id: String,
    patreon_tier_id: usize,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize, Debug)]
struct NonPremiumResponse {
    premium: bool,
}

pub async fn listen(
    server_addr: &str,
    data: Arc<RwLock<HashMap<String, Vec<Entitlement>>>>,
    last_poll_time: Arc<RwLock<DateTime<Utc>>>,
) {
    let addr = SocketAddr::from_str(server_addr).unwrap();

    let ping = warp::path("ping").and_then(ping);

    let data = warp::any().map(move || Arc::clone(&data));
    let last_poll_time = warp::any().map(move || Arc::clone(&last_poll_time));

    let is_premium = warp::path("ispremium")
        .and(data.clone())
        .and(warp::query::<HashMap<String, String>>())
        .and_then(is_premium);

    let all = warp::path("all")
        .and(data.clone())
        .and(last_poll_time.clone())
        .and(warp::query())
        .and_then(all_patrons);

    let count = warp::path("count").and(data.clone()).and_then(patron_count);

    warp::serve(ping.or(is_premium).or(all).or(count))
        .run(addr)
        .await;
}

async fn ping() -> Result<Json, warp::Rejection> {
    Ok(reply::json(&PingResponse { success: true }))
}

async fn is_premium(
    patrons: Arc<RwLock<HashMap<String, Vec<Entitlement>>>>,
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
    let highest_tier_id = tiers.iter().max_by_key(|t| t.tier_id()).unwrap().tier_id();
    let mut guild_highest_tier: Option<&Entitlement> = None;
    let mut user_id: Option<&str> = None;

    let patrons = patrons.read().unwrap();

    // any so we stop at the first true
    // we need to find the highest tier, so we need to only break at
    ids.any(|id| {
        if let Some(entitlements) = patrons.get(id) {
            let top_entitlement = entitlements
                .iter()
                .max_by_key(|entitlement| entitlement.tier.tier_id());
            if let Some(top_entitlement) = top_entitlement {
                match guild_highest_tier {
                    None => {
                        guild_highest_tier = Some(&top_entitlement);
                        user_id = Some(id);
                    }
                    Some(current_tier) => {
                        if top_entitlement.tier.tier_id() > current_tier.tier.tier_id() {
                            guild_highest_tier = Some(&top_entitlement);
                            user_id = Some(id);
                        }
                    }
                }
            }

            return guild_highest_tier.map(|e| e.tier.tier_id()) == Some(highest_tier_id);
        } else {
            false
        }
    });

    let response_data: Json = if let Some(entitlement) = guild_highest_tier {
        reply::json(&PremiumResponse {
            premium: true,
            tier: entitlement.tier.tier_id(),
            user_id: user_id.unwrap().to_owned(),
            patreon_tier_id: entitlement.patreon_tier_id,
            expires_at: entitlement.expires_at,
        })
    } else {
        reply::json(&NonPremiumResponse { premium: false })
    };

    Ok(reply::with_status(response_data, StatusCode::OK))
}

async fn patron_count(
    patrons: Arc<RwLock<HashMap<String, Vec<Entitlement>>>>,
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
    patrons: Arc<RwLock<HashMap<String, Vec<Entitlement>>>>,
    last_poll_time: Arc<RwLock<DateTime<Utc>>>,
    query: HashMap<String, String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let mut patrons = patrons.read().unwrap().clone();

    let legacy_only = query.get("legacy_only").map_or(false, |v| v == "true");
    if legacy_only {
        for (_, entitlements) in patrons.iter_mut() {
            entitlements.retain(|entitlement| entitlement.is_legacy);
        }

        // Remove now-empty entries
        for (k, v) in patrons.clone().iter() {
            if v.is_empty() {
                patrons.remove(k);
            }
        }
    }

    let last_poll_time = last_poll_time.read().unwrap().clone();
    let last_poll_time_value = match serde_json::to_value(&last_poll_time) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to serialize last poll time: {}", e);

            return Ok(reply::with_status(
                reply::json(&json!({
                    "error": "Unable to serialize last poll time"
                })),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let value = match serde_json::to_value(&patrons) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to serialize patrons: {}", e);

            return Ok(reply::with_status(
                reply::json(&json!({
                    "error": "Unable to serialize patrons map"
                })),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    Ok(reply::with_status(
        reply::json(&json!({
            "last_poll_time": last_poll_time_value,
            "entitlements": value
        })),
        StatusCode::OK,
    ))
}
