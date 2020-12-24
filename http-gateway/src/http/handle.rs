use warp::reply::Json;
use crate::http::Server;
use std::sync::Arc;
use ed25519_dalek::{Verifier, Signature, PublicKey};
use warp::hyper::body::Bytes;
use crate::Error;
use warp::Rejection;
use model::interaction::{Interaction, InteractionType, InteractionResponse};
use common::event_forwarding::Command;
use serde_json::value::RawValue;
use std::str;
use model::Snowflake;

pub async fn handle(
    bot_id: Snowflake,
    server: Arc<Server>,
    signature: Signature,
    timestamp: String,
    body: Bytes,
) -> Result<Json, Rejection> {
    let timestamp = (&timestamp[..]).as_bytes();
    let body = &body[..];

    let body_with_timestamp: Vec<u8> = timestamp.iter().copied().chain(body.iter().copied()).collect();

    let public_key = get_public_key(server.clone(), bot_id).await.map_err(warp::reject::custom)?;

    if let Err(e) = public_key.verify(&body_with_timestamp[..], &signature) {
        return Err(Error::InvalidSignature(e).into());
    }

    // TODO: Log errors
    let interaction: Interaction = serde_json::from_slice(&body)
        .map_err(Error::JsonError)
        .map_err(warp::reject::custom)?;

    match interaction.r#type {
        InteractionType::Ping => {
            let response = InteractionResponse::new_pong();
            Ok(warp::reply::json(&response))
        }

        _ => {
            match interaction.guild_id { // Should never be None
                Some(guild_id) => match forward(server, bot_id, guild_id, body).await {
                    Ok(_) => {
                        let response = InteractionResponse::new_ack_with_source();
                        Ok(warp::reply::json(&response))
                    }
                    Err(e) => {
                        // TODO: Proper logging
                        eprintln!("Error occurred while forwarding command: {}", e);
                        Err(warp::reject::custom(e))
                    }
                }
                None => Err(warp::reject::custom(Error::MissingGuildId)),
            }
        }
    }
}

async fn get_public_key(server: Arc<Server>, bot_id: Snowflake) -> Result<PublicKey, Error> {
    if bot_id == server.config.main_bot_id {
        Ok(server.config.main_public_key)
    } else {
        match server.database.whitelabel_keys.get(bot_id).await {
            Ok(raw) => {
                let mut bytes = [0u8; 32];
                hex::decode_to_slice(raw.as_bytes(), &mut bytes).map_err(Error::InvalidSignatureFormat)?;

                PublicKey::from_bytes(&bytes).map_err(Error::InvalidSignature)
            }
            Err(e) => Err(Error::DatabaseError(e))
        }
    }
}

pub async fn forward(server: Arc<Server>, bot_id: Snowflake, guild_id: Snowflake, data: &[u8]) -> Result<(), Error> {
    let json = str::from_utf8(data).map_err(Error::Utf8Error)?.to_owned();

    let token = get_token(server.clone(), bot_id).await?;
    let is_whitelabel = bot_id == server.config.main_bot_id;

    let wrapped = Command {
        bot_token: &token,
        bot_id: bot_id.0,
        is_whitelabel,
        data: RawValue::from_string(json).map_err(Error::JsonError)?,
    };

    let mut req = server.http_client.clone()
        .post(&server.config.worker_svc_uri[..])
        .header("x-guild-id", guild_id.0.to_string())
        .json(&wrapped);

    // apply sticky cookie header
    // although whitelabel bots run on shard 0 only, we still treat them in the regular fashion to
    // ensure a good spread of events across all workers, to prevent all events going to the worker
    // assigned to shard 0 only.
    let shard_id = calculate_shard_id(guild_id, server.config.shard_count);

    let cookie = server.cookies.read().await;
    if let Some(cookie) = cookie.get(&shard_id) {
        let value = format!("{}={}", server.config.worker_sticky_cookie, cookie);
        req = req.header(reqwest::header::COOKIE, value);
    }
    drop(cookie); // drop here so we can write safely later

    let res = req.send()
        .await
        .map_err(Error::ReqwestError)?;

    if let Some(cookie) = res.cookies().find(|c| c.name() == &*server.config.worker_sticky_cookie) {
        let mut cookies = server.cookies.write().await;
        cookies.insert(shard_id, Box::from(cookie.value()));
    }

    Ok(())
}

// Returns tuple of (token,is_whitelabel)
async fn get_token<'a>(server: Arc<Server>, bot_id: Snowflake) -> Result<Box<str>, Error> {
    // Check if public bot
    if server.config.main_bot_id == bot_id {
        let token = server.config.main_bot_token.clone();
        return Ok(token);
    }

    let bot = server.database.whitelabel.get_bot_by_id(bot_id).await.map_err(Error::DatabaseError)?;
    match bot {
        Some(bot) => Ok(bot.token.into_boxed_str()),
        None => Err(Error::TokenNotFound(bot_id)),
    }
}

fn calculate_shard_id(guild_id: Snowflake, shard_count: u16) -> u16 {
    ((guild_id.0 >> 22) % (shard_count as u64)) as u16
}