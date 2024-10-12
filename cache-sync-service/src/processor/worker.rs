use std::{sync::Arc, time::Instant};

use crate::Result;
use cache::Cache;
use event_stream::Consumer;
use lazy_static::lazy_static;
use model::{
    guild::{Guild, Member},
    Snowflake,
};
use prometheus::{
    register_counter_vec, register_gauge, register_histogram_vec, CounterVec, Gauge, HistogramVec,
};
use serde_json::value::RawValue;
use sharder::payloads::{event::Event, Dispatch};
use tracing::{debug, error, trace};

lazy_static! {
    static ref EVENT_COUNTER: CounterVec = register_counter_vec!(
        "cache_events",
        "Number of cache events processed",
        &["event_type"]
    )
    .unwrap();
    static ref CONCURRENT_EVENTS_GUAGE: Gauge = register_gauge!(
        "concurrent_events",
        "Number of events being processed concurrently"
    )
    .unwrap();
    static ref TIME_TO_CACHE: HistogramVec = register_histogram_vec!(
        "time_to_cache",
        "Time taken to cache an event",
        &["event_type"]
    )
    .unwrap();
}

pub struct Worker<C: Cache> {
    id: usize,
    consumer: Arc<Consumer>,
    cache: Arc<C>,
}

impl<C: Cache> Worker<C> {
    pub fn new(id: usize, consumer: Arc<Consumer>, cache: Arc<C>) -> Self {
        Self {
            id,
            consumer,
            cache,
        }
    }

    pub async fn run(&self) {
        debug!(%self.id, "Starting worker");

        loop {
            let ev = match self.consumer.recv().await {
                Ok(ev) => ev,
                Err(e) => {
                    error!(error = %e, "Failed to receive event");
                    continue;
                }
            };

            debug!(%ev.bot_id, "Received event");

            CONCURRENT_EVENTS_GUAGE.inc();

            if let Err(e) = self.handle_event(ev.event).await {
                error!(error = %e, "Failed to handle event.");
                CONCURRENT_EVENTS_GUAGE.dec();
                continue;
            }

            CONCURRENT_EVENTS_GUAGE.dec();
        }
    }

    async fn handle_event(&self, raw: Box<RawValue>) -> Result<()> {
        let payload: Dispatch = serde_json::from_str(raw.get())?;

        trace!(?payload, "Received event");

        let event_name = payload.data.to_string();
        EVENT_COUNTER
            .with_label_values(&[event_name.as_str()])
            .inc();

        let now = Instant::now();

        let mut cachable = true;
        match payload.data {
            Event::ChannelCreate(c) => self.cache.store_channel(c).await?,
            Event::ChannelUpdate(c) => self.cache.store_channel(c).await?,
            Event::ChannelDelete(c) => self.cache.delete_channel(c.id).await?,
            Event::ThreadCreate(t) => self.cache.store_channel(t).await?,
            Event::ThreadUpdate(t) => {
                if t.thread_metadata
                    .as_ref()
                    .map(|m| m.archived)
                    .unwrap_or(false)
                {
                    self.cache.delete_channel(t.id).await?
                } else {
                    self.cache.store_channel(t).await?
                }
            }
            Event::ThreadDelete(t) => self.cache.delete_channel(t.id).await?,
            Event::GuildCreate(mut g) => {
                apply_guild_id_to_channels(&mut g);
                self.cache.store_guild(g).await?;
            }
            Event::GuildUpdate(mut g) => {
                apply_guild_id_to_channels(&mut g);
                self.cache.store_guild(g).await?;
            }
            Event::GuildDelete(g) => self.cache.delete_guild(g.id).await?,
            // When removing members, also remove the user, as it's too expensive to check if the user is in another guild.
            // It is cheaper to just fetch the user again later.
            Event::GuildBanAdd(ev) => self.remove_member_and_user(ev.user.id, ev.guild_id).await?,
            Event::GuildMemberRemove(ev) => {
                self.remove_member_and_user(ev.user.id, ev.guild_id).await?
            }
            Event::GuildMemberUpdate(ev) => {
                self.cache
                    .store_member(
                        Member {
                            user: Some(ev.user),
                            nick: ev.nick,
                            roles: ev.roles,
                            joined_at: ev.joined_at,
                            premium_since: ev.premium_since,
                            deaf: false,
                            mute: false,
                        },
                        ev.guild_id,
                    )
                    .await?
            }
            Event::GuildMembersChunk(ev) => {
                self.cache.store_members(ev.members, ev.guild_id).await?
            }
            Event::GuildRoleCreate(ev) => self.cache.store_role(ev.role, ev.guild_id).await?,
            Event::GuildRoleUpdate(ev) => self.cache.store_role(ev.role, ev.guild_id).await?,
            Event::GuildRoleDelete(ev) => self.cache.delete_role(ev.role_id).await?,
            Event::UserUpdate(ev) => self.cache.store_user(ev).await?,
            Event::GuildEmojisUpdate(ev) => self.cache.store_emojis(ev.emojis, ev.guild_id).await?,
            _ => {
                cachable = false;
            }
        };

        if cachable {
            TIME_TO_CACHE
                .with_label_values(&[event_name.as_str()])
                .observe(now.elapsed().as_secs_f64());
        }

        Ok(())
    }

    async fn remove_member_and_user(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        self.cache.delete_member(user_id, guild_id).await?;
        self.cache.delete_user(user_id).await?;

        Ok(())
    }
}

fn apply_guild_id_to_channels(guild: &mut Guild) {
    guild.channels.as_mut().map(|channels| {
        channels
            .iter_mut()
            .for_each(|c| c.guild_id = Some(guild.id))
    });

    guild
        .threads
        .as_mut()
        .map(|threads| threads.iter_mut().for_each(|t| t.guild_id = Some(guild.id)));
}
