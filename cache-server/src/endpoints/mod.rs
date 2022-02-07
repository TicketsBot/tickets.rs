use crate::Response;
use axum::extract::{Extension, Json as JsonBody, Path};
use axum::response::Json;
use cache::MemoryCache;
use hyper::StatusCode;
use std::sync::Arc;
use model::channel::Channel;
use model::guild::{Guild, Member, Role};
use model::user::User;

// TODO: Cache options
pub async fn get_guild(
    Path(guild_id): Path<u64>,
    cache: Extension<Arc<MemoryCache>>,
) -> (StatusCode, Json<Response<Guild>>) {
    match cache.0.get_guild(guild_id.into(), all_opts()) {
        Ok(Some(guild)) => (StatusCode::OK, Response::Found(guild).into()),
        Ok(None) => (StatusCode::NOT_FOUND, Response::CacheMiss.into()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Response::error(format!("{}", e)).into()),
    }
}

pub async fn get_guild_channels(
    Path(guild_id): Path<u64>,
    cache: Extension<Arc<MemoryCache>>,
) -> (StatusCode, Json<Response<Vec<Channel>>>) {
    match cache.0.get_guild_channels(guild_id.into()) {
        Ok(channels) => (StatusCode::OK, Response::Found(channels).into()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Response::error(format!("{}", e)).into()),
    }
}

pub async fn get_guild_channel(
    Path((guild_id, channel_id)): Path<(u64, u64)>,
    cache: Extension<Arc<MemoryCache>>,
) -> (StatusCode, Json<Response<Channel>>) {
    match cache.0.get_channel(channel_id.into(), Some(guild_id.into())) {
        Ok(Some(channel)) => (StatusCode::OK, Response::Found(channel).into()),
        Ok(None) => (StatusCode::NOT_FOUND, Response::CacheMiss.into()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Response::error(format!("{}", e)).into()),
    }
}

pub async fn get_dm_channel(
    Path(channel_id): Path<u64>,
    cache: Extension<Arc<MemoryCache>>,
) -> (StatusCode, Json<Response<Channel>>) {
    match cache.0.get_channel(channel_id.into(), None) {
        Ok(Some(channel)) => (StatusCode::OK, Response::Found(channel).into()),
        Ok(None) => (StatusCode::NOT_FOUND, Response::CacheMiss.into()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Response::error(format!("{}", e)).into()),
    }
}

pub async fn get_member(
    Path((guild_id, user_id)): Path<(u64, u64)>,
    cache: Extension<Arc<MemoryCache>>,
) -> (StatusCode, Json<Response<Member>>) {
    match cache.0.get_member(user_id.into(), guild_id.into()) {
        Ok(Some(member)) => (StatusCode::OK, Response::Found(member).into()),
        Ok(None) => (StatusCode::NOT_FOUND, Response::CacheMiss.into()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Response::error(format!("{}", e)).into()),
    }
}

pub async fn get_role(
    Path((guild_id, role_id)): Path<(u64, u64)>,
    cache: Extension<Arc<MemoryCache>>,
) -> (StatusCode, Json<Response<Role>>) {
    match cache.0.get_role(role_id.into(), guild_id.into()) {
        Ok(Some(role)) => (StatusCode::OK, Response::Found(role).into()),
        Ok(None) => (StatusCode::NOT_FOUND, Response::CacheMiss.into()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Response::error(format!("{}", e)).into()),
    }
}

pub async fn get_guild_roles(
    Path(guild_id): Path<u64>,
    cache: Extension<Arc<MemoryCache>>,
) -> (StatusCode, Json<Response<Vec<Role>>>) {
    match cache.0.get_guild_roles(guild_id.into()) {
        Ok(roles) => (StatusCode::OK, Response::Found(roles).into()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Response::error(format!("{}", e)).into()),
    }
}

pub async fn post_member(
    Path(guild_id): Path<u64>,
    JsonBody(member): JsonBody<Member>,
    cache: Extension<Arc<MemoryCache>>,
) -> (StatusCode, Json<Response<()>>) {
    match cache.store_member(member, guild_id.into()) {
        Ok(_) => (StatusCode::OK, Response::success().into()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.into())
    }
}

pub async fn post_members(
    Path(guild_id): Path<u64>,
    JsonBody(member): JsonBody<Vec<Member>>,
    cache: Extension<Arc<MemoryCache>>,
) -> (StatusCode, Json<Response<()>>) {
    match cache.store_members(member, guild_id.into()) {
        Ok(_) => (StatusCode::OK, Response::success().into()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.into())
    }
}

pub async fn get_user(
    Path(user_id): Path<u64>,
    cache: Extension<Arc<MemoryCache>>,
) -> (StatusCode, Json<Response<Role>>) {
    match cache.0.get_user(user_id.into()) {
        Ok(Some(user)) => (StatusCode::OK, Response::Found(user).into()),
        Ok(None) => (StatusCode::NOT_FOUND, Response::CacheMiss.into()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Response::error(format!("{}", e)).into()),
    }
}

pub async fn post_user(
    Path(user_id): Path<u64>, // user_id param not actually used, just separating bulk route
    JsonBody(user): JsonBody<User>,
    cache: Extension<Arc<MemoryCache>>,
) -> (StatusCode, Json<Response<()>>) {
    match cache.store_user(user) {
        Ok(_) => (StatusCode::OK, Response::success().into()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.into())
    }
}

pub async fn post_users(
    JsonBody(users): JsonBody<Vec<User>>,
    cache: Extension<Arc<MemoryCache>>,
) -> (StatusCode, Json<Response<()>>) {
    match cache.store_users(users) {
        Ok(_) => (StatusCode::OK, Response::success().into()),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.into())
    }
}

fn all_opts() -> cache::Options {
    cache::Options {
        users: true,
        guilds: true,
        members: true,
        channels: true,
        roles: true,
        emojis: true,
        voice_states: false,
    }
}
