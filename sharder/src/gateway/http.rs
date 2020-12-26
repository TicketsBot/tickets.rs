use crate::{Shard, GatewayError};
use common::event_forwarding;
use std::sync::Arc;
use std::time::Duration;
use crate::gateway::worker_response::WorkerResponse;
use crate::gateway::payloads::event::Event;
use model::Snowflake;
use tokio::time::delay_for;

impl Shard {
    pub async fn forward_event(self: Arc<Self>, event: event_forwarding::Event<'_>, guild_id: Option<Snowflake>) -> Result<WorkerResponse, GatewayError> {
        let uri = &*self.config.worker_svc_uri;

        // reqwest::Client uses Arcs internally, meaning this method clones the same client but
        // allows us to make use of connection pooling
        let mut req = self.http_client.clone()
            .post(uri)
            .json(&event);

        if let Some(guild_id) = guild_id {
            let header_name = &*self.config.sticky_cookie;
            req = req.header(header_name, guild_id.0);
        }

        let cookie = self.cookie.read().await;
        if let Some(cookie) = &*cookie {
            let value = format!("{}={}", self.config.sticky_cookie, cookie);
            req = req.header(reqwest::header::COOKIE, value);
        }
        drop(cookie); // drop here so we can write later

        let res = req.send()
            .await
            .map_err(GatewayError::ReqwestError)?;

        if let Some(cookie) = res.cookies().find(|c| c.name() == &*self.config.sticky_cookie) {
            self.log(format!("Got new session cookie: {}", cookie.value()));
            *self.cookie.write().await = Some(Box::from(cookie.value()));
        }

        serde_json::
        res.json()
            .await
            .map_err(GatewayError::ReqwestError)
    }

    pub fn build_http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .gzip(cfg!(feature = "compression"))
            .build()
            .expect("build_http_client")
    }

    pub fn get_guild_id(event: &Event) -> Option<Snowflake> {
        match event {
            Event::ChannelCreate(data) => data.guild_id,
            Event::ChannelUpdate(data) => data.guild_id,
            Event::ChannelDelete(data) => data.guild_id,
            Event::ChannelPinsUpdate(data) => data.guild_id,
            Event::GuildCreate(data) => Some(data.id),
            Event::GuildUpdate(data) => Some(data.id),
            Event::GuildDelete(data) => Some(data.id),
            Event::GuildBanAdd(data) => Some(data.guild_id),
            Event::GuildBanRemove(data) => Some(data.guild_id),
            Event::GuildEmojisUpdate(data) => Some(data.guild_id),
            Event::GuildIntegrationsUpdate(data) => Some(data.guild_id),
            Event::GuildMemberAdd(data) => Some(data.guild_id),
            Event::GuildMemberRemove(data) => Some(data.guild_id),
            Event::GuildMemberUpdate(data) => Some(data.guild_id),
            Event::GuildMembersChunk(data) => Some(data.guild_id),
            Event::GuildRoleCreate(data) => Some(data.guild_id),
            Event::GuildRoleUpdate(data) => Some(data.guild_id),
            Event::GuildRoleDelete(data) => Some(data.guild_id),
            Event::InviteCreate(data) => data.guild_id,
            Event::InviteDelete(data) => data.guild_id,
            Event::MessageCreate(data) => data.guild_id,
            Event::MessageUpdate(data) => data.guild_id,
            Event::MessageDelete(data) => data.guild_id,
            Event::MessageDeleteBulk(data) => data.guild_id,
            Event::MessageReactionAdd(data) => data.guild_id,
            Event::MessageReactionRemove(data) => data.guild_id,
            Event::MessageReactionRemoveAll(data) => data.guild_id,
            Event::MessageReactionRemoveEmoji(data) => data.guild_id,
            Event::PresenceUpdate(data) => data.guild_id,
            Event::TypingStart(data) => data.guild_id,
            Event::VoiceStateUpdate(data) => data.guild_id,
            Event::VoiceServerUpdate(data) => Some(data.guild_id),
            Event::WebhookUpdate(data) => Some(data.guild_id),
            _ => None
        }
    }

    pub fn start_reset_cookie_loop(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                delay_for(Duration::from_secs(180));
                *self.cookie.write().await = None;
            }
        });
    }
}