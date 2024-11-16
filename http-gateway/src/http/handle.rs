use crate::http::Server;
use crate::Error;
use cache::Cache;
use common::event_forwarding::ForwardedInteraction;
use database::WhitelabelBot;
use ed25519_dalek::{PublicKey, Signature, Verifier};
use model::guild::Member;
use model::interaction::{
    ApplicationCommandInteraction, Interaction, InteractionResponse, InteractionType,
    MessageComponentInteraction,
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

    let whitelabel_bot: Option<WhitelabelBot> = if bot_id == server.config.public_bot_id {
        None
    } else {
        match server.database.whitelabel.get_bot_by_id(bot_id).await {
            Ok(bot @ Some(_)) => bot,
            Ok(None) => return Err(warp::reject::custom(Error::BotNotFound(bot_id))),
            Err(e) => return Err(warp::reject::custom(Error::DatabaseError(e))),
        }
    };

    let public_key = match &whitelabel_bot {
        Some(bot) => {
            let mut bytes = [0u8; 32];
            hex::decode_to_slice(bot.public_key.clone().as_bytes(), &mut bytes)
                .map_err(Error::InvalidSignatureFormat)?;

            PublicKey::from_bytes(&bytes)
                .map_err(Error::InvalidSignature)
                .map_err(warp::reject::custom)?
        }
        None => server.config.public_public_key,
    };

    if let Err(e) = public_key.verify(&body_with_timestamp[..], &signature) {
        return Err(Error::InvalidSignature(e).into());
    }

    let token = match whitelabel_bot {
        Some(bot) => bot.token,
        None => server.config.public_token.clone(),
    };

    let interaction: Interaction = serde_json::from_slice(&body[..])
        .map_err(Error::JsonError)
        .map_err(warp::reject::custom)?;

    match interaction {
        Interaction::Ping(data) => {
            if data.application_id != bot_id {
                return Err(Error::InvalidApplicationId(data.application_id, bot_id).into());
            }

            let response = InteractionResponse::new_pong();
            Ok(warp::reply::json(&response).into_response())
        }

        Interaction::ApplicationCommand(data) => {
            let interaction_type = data.r#type;

            if let Some(guild_id) = data.guild_id {
                let server = Arc::clone(&server);
                tokio::spawn(async move {
                    if let Err(e) = cache_resolved(server, *data, guild_id).await {
                        eprintln!("error caching resolved: {}", e);
                    }
                });
            }

            let res_body = forward(server, bot_id, token, interaction_type, &body[..])
                .await
                .map_err(warp::reject::custom)?;

            Ok(Response::new(Body::from(res_body)))
        }

        Interaction::MessageComponent(data) => {
            let interaction_type = data.r#type;

            if let Some(guild_id) = data.guild_id {
                let server = Arc::clone(&server);
                tokio::spawn(async move {
                    if let Err(e) =
                        cache_message_component_interaction(server, *data, guild_id).await
                    {
                        eprintln!("error caching resolved: {}", e);
                    }
                });
            }

            let res_body = forward(server, bot_id, token, interaction_type, &body[..])
                .await
                .map_err(warp::reject::custom)?;

            Ok(Response::new(Body::from(res_body)))
        }

        Interaction::ApplicationCommandAutoComplete(data) => {
            let res_body = forward(server, bot_id, token, data.r#type, &body[..])
                .await
                .map_err(warp::reject::custom)?;

            Ok(Response::new(Body::from(res_body)))
        }

        Interaction::ModalSubmit(data) => {
            let res_body = forward(server, bot_id, token, data.r#type, &body[..])
                .await
                .map_err(warp::reject::custom)?;

            Ok(Response::new(Body::from(res_body)))
        }

        _ => Err(warp::reject::custom(Error::UnsupportedInteractionType)),
    }
}

pub async fn forward<T: Cache>(
    server: Arc<Server<T>>,
    bot_id: Snowflake,
    token: String,
    interaction_type: InteractionType,
    data: &[u8],
) -> Result<Bytes, Error> {
    let json = str::from_utf8(data).map_err(Error::Utf8Error)?.to_owned();

    let is_whitelabel = bot_id != server.config.public_bot_id;

    let wrapped = ForwardedInteraction {
        bot_token: token.as_str(),
        bot_id: bot_id.0,
        is_whitelabel,
        interaction_type,
        data: RawValue::from_string(json).map_err(Error::JsonError)?,
    };

    let req = server
        .http_client
        .clone()
        .post(&*server.config.get_svc_uri())
        .json(&wrapped);

    let res = req.send().await.map_err(Error::ReqwestError)?;
    let res_body = res.bytes().await.map_err(Error::ReqwestError)?;

    Ok(res_body)
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

    Ok(())
}

async fn cache_message_component_interaction<T: Cache>(
    server: Arc<Server<T>>,
    interaction: MessageComponentInteraction,
    guild_id: Snowflake,
) -> Result<(), Error> {
    if let Some(member) = interaction.member {
        if let Some(ref user) = member.user {
            server.cache.store_user(user.clone()).await?;
        }

        server.cache.store_member(member, guild_id).await?;
    } else if let Some(user) = interaction.user {
        server.cache.store_user(user).await?;
    }

    Ok(())
}
