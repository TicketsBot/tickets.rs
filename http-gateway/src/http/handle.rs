use crate::http::Server;
use crate::Error;
use cache::Cache;
use common::event_forwarding::ForwardedInteraction;
use ed25519_dalek::{PublicKey, Signature, Verifier};
use model::guild::{Member, Role};
use model::interaction::{
    ApplicationCommandInteraction, Interaction, InteractionApplicationCommandCallbackData,
    InteractionResponse, InteractionType,
};
use model::user::User;
use model::Snowflake;
use serde_json::value::RawValue;
use std::str;
use std::sync::Arc;
use warp::hyper::body::Bytes;
use warp::hyper::Body;
use warp::{reply::Response, Rejection, Reply};

pub async fn handle<T: Cache>(
    bot_id: Snowflake,
    server: Arc<Server<T>>,
    signature: Signature,
    timestamp: String,
    body: Bytes,
) -> Result<Response, Rejection> {
    let timestamp = (&timestamp[..]).as_bytes();
    let body_slice = &body[..];

    let body_with_timestamp: Vec<u8> = timestamp
        .iter()
        .copied()
        .chain(body_slice.iter().copied())
        .collect();

    let public_key = get_public_key(server.clone(), bot_id)
        .await
        .map_err(warp::reject::custom)?;

    if let Err(e) = public_key.verify(&body_with_timestamp[..], &signature) {
        return Err(Error::InvalidSignature(e).into());
    }

    let interaction: Interaction = serde_json::from_slice(&body[..])
        .map_err(Error::JsonError)
        .map_err(warp::reject::custom)?;

    match interaction {
        Interaction::Ping(_) => {
            let response = InteractionResponse::new_pong();
            Ok(warp::reply::json(&response).into_response())
        }

        Interaction::ApplicationCommand(data) => match data.guild_id {
            Some(guild_id) => {
                let interaction_type = data.r#type;

                {
                    let server = Arc::clone(&server);
                    tokio::spawn(async move {
                        if let Err(e) = cache_resolved(server, data, guild_id).await {
                            eprintln!("error caching resolved: {}", e);
                        }
                    });
                }

                let res_body = forward(server, bot_id, guild_id, interaction_type, &body[..])
                    .await
                    .map_err(warp::reject::custom)?;
                Ok(Response::new(Body::from(res_body)))
            }
            None => Ok(warp::reply::json(&get_missing_guild_id_response()).into_response()),
        },

        Interaction::Button(data) => match data.guild_id {
            Some(guild_id) => {
                let res_body = forward(server, bot_id, guild_id, data.r#type, &body[..])
                    .await
                    .map_err(warp::reject::custom)?;
                Ok(Response::new(Body::from(res_body)))
            }
            None => Ok(warp::reply::json(&get_missing_guild_id_response()).into_response()),
        }, //_ => Err(warp::reject::custom(Error::UnsupportedInteractionType))
    }
}

fn get_missing_guild_id_response() -> InteractionResponse {
    let data = InteractionApplicationCommandCallbackData {
        tts: None,
        content: Box::from(
            "Commands in DMs are not currently supported. Please run this command in a server.",
        ),
        embeds: None,
        allowed_mentions: None,
        flags: 0,
    };

    InteractionResponse::new_channel_message_with_source(data)
}

async fn get_public_key<T: Cache>(
    server: Arc<Server<T>>,
    bot_id: Snowflake,
) -> Result<PublicKey, Error> {
    if bot_id == server.config.public_bot_id {
        Ok(server.config.public_public_key)
    } else {
        match server.database.whitelabel_keys.get(bot_id).await {
            Ok(raw) => {
                let mut bytes = [0u8; 32];
                hex::decode_to_slice(raw.as_bytes(), &mut bytes)
                    .map_err(Error::InvalidSignatureFormat)?;

                PublicKey::from_bytes(&bytes).map_err(Error::InvalidSignature)
            }
            Err(e) => Err(Error::DatabaseError(e)),
        }
    }
}

pub async fn forward<T: Cache>(
    server: Arc<Server<T>>,
    bot_id: Snowflake,
    guild_id: Snowflake,
    interaction_type: InteractionType,
    data: &[u8],
) -> Result<Bytes, Error> {
    let json = str::from_utf8(data).map_err(Error::Utf8Error)?.to_owned();

    let token = get_token(server.clone(), bot_id).await?;
    let is_whitelabel = bot_id == server.config.public_bot_id;

    let wrapped = ForwardedInteraction {
        bot_token: &token,
        bot_id: bot_id.0,
        is_whitelabel,
        interaction_type,
        data: RawValue::from_string(json).map_err(Error::JsonError)?,
    };

    let req = server
        .http_client
        .clone()
        .post(&*server.config.get_svc_uri())
        .header("x-guild-id", guild_id.0.to_string())
        .json(&wrapped);

    let res = req.send().await.map_err(Error::ReqwestError)?;
    let res_body = res.bytes().await.map_err(Error::ReqwestError)?;

    Ok(res_body)
}

// Returns tuple of (token,is_whitelabel)
async fn get_token<T: Cache>(server: Arc<Server<T>>, bot_id: Snowflake) -> Result<Box<str>, Error> {
    // Check if public bot
    if server.config.public_bot_id == bot_id {
        let token = server.config.public_token.clone();
        return Ok(token);
    }

    let bot = server
        .database
        .whitelabel
        .get_bot_by_id(bot_id)
        .await
        .map_err(Error::DatabaseError)?;
    match bot {
        Some(bot) => Ok(bot.token.into_boxed_str()),
        None => Err(Error::TokenNotFound(bot_id)),
    }
}

async fn cache_resolved<T: Cache>(
    server: Arc<Server<T>>,
    interaction: ApplicationCommandInteraction,
    guild_id: Snowflake,
) -> Result<(), Error> {
    let mut users: Vec<User> = interaction
        .data
        .resolved
        .users
        .into_iter()
        .map(|(_, user)| user)
        .collect();

    let mut members: Vec<Member> = interaction
        .data
        .resolved
        .members
        .into_iter()
        .map(|(_, member)| member)
        .collect();

    let roles: Vec<Role> = interaction
        .data
        .resolved
        .roles
        .into_iter()
        .map(|(_, role)| role)
        .collect();

    // Don't cache channels since data is extremely basic

    if let Some(member) = interaction.member {
        if let Some(ref user) = member.user {
            users.push(user.clone());
        }

        members.push(member);
    } else if let Some(user) = interaction.user {
        users.push(user);
    }

    server.cache.store_users(users).await?;
    server.cache.store_members(members, guild_id).await?;
    server.cache.store_roles(roles, guild_id).await?;

    Ok(())
}
