use warp::reply::Json;
use crate::http::Server;
use std::sync::Arc;
use ed25519_dalek::{Verifier, Signature};
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
    if let Err(e) = server.config.main_public_key.verify(&body_with_timestamp[..], &signature) {
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

pub async fn forward(server: Arc<Server>, bot_id: Snowflake, guild_id: Snowflake, data: &[u8]) -> Result<(), Error> {
    let json = str::from_utf8(data).map_err(Error::Utf8Error)?.to_owned();

    let (token, is_whitelabel) = get_token(server.clone(), bot_id).await?;

    let wrapped = Command {
        bot_token: &token,
        bot_id: bot_id.0,
        is_whitelabel,
        data: RawValue::from_string(json).map_err(Error::JsonError)?,
    };

    server.http_client.clone()
        .post(&server.config.worker_svc_uri[..])
        .header("x-guild-id", guild_id.0.to_string())
        .json(&wrapped)
        .send()
        .await
        .map_err(Error::ReqwestError)?;

    Ok(())
}

// Returns tuple of (token,is_whitelabel)
async fn get_token<'a>(server: Arc<Server>, bot_id: Snowflake) -> Result<(Box<str>, bool), Error> {
    // Check if public bot
    if server.config.main_bot_id == bot_id {
        let token = server.config.main_bot_token.clone();
        return Ok((token, false));
    }

    let bot = server.database.whitelabel.get_bot_by_id(bot_id).await.map_err(Error::DatabaseError)?;
    match bot {
        Some(bot) => Ok((bot.token.into_boxed_str(), true)),
        None => Err(Error::TokenNotFound(bot_id)),
    }
}