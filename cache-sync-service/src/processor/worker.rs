use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::Result;
use cache::{postgres::Worker as CacheWorker, PostgresCache};
use event_stream::Consumer;
use lazy_static::lazy_static;
use model::{
    guild::{Guild, Member},
    Snowflake,
};
use prometheus::{
    register_counter_vec, register_gauge, register_histogram, register_histogram_vec, CounterVec, Gauge, Histogram, HistogramVec
};
use serde_json::value::RawValue;
use sharder::payloads::{event::Event, Dispatch};
use tokio::{task::JoinSet, time::timeout};
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
    static ref REAL_BATCH_SIZE_HISTOGRAM: Histogram = register_histogram!(
        "real_batch_size",
        "Real batch size of events",
    )
    .unwrap();
    static ref EVENT_SIZE_HISTOGRAM: HistogramVec = register_histogram_vec!(
        "event_size",
        "Size of events",
        &["event_type"]
    )
    .unwrap();
}

pub struct Worker {
    id: usize,
    batch_size: usize,
    consumer: Arc<Consumer>,
    cache: Arc<PostgresCache>,
}

impl Worker {
    pub fn new(
        id: usize,
        batch_size: usize,
        consumer: Arc<Consumer>,
        cache: Arc<PostgresCache>,
    ) -> Self {
        Self {
            id,
            batch_size,
            consumer,
            cache,
        }
    }

    pub async fn run(&self) {
        debug!(%self.id, "Starting worker");

        'event_loop: loop {
            let mut batch = Vec::with_capacity(self.batch_size);
            let fst = match self.consumer.recv().await {
                Ok(ev) => ev,
                Err(e) => {
                    error!(error = %e, "Failed to receive event");
                    continue;
                }
            };

            batch.push(fst);

            for _ in 0..self.batch_size-1 {
                match timeout(Duration::from_secs(1), self.consumer.recv()).await {
                    Ok(Ok(ev)) => batch.push(ev),
                    Ok(Err(e)) => {
                        error!(error = %e, "Failed to receive event");
                        continue 'event_loop;
                    }
                    Err(_) => break, // Timeout
                }
            }

            debug!(size = batch.len(), "Received batch");

            REAL_BATCH_SIZE_HISTOGRAM.observe(batch.len() as f64);

            // CONCURRENT_EVENTS_GUAGE.add(batch.len());

            let cache_worker = match self.cache.build_worker().await {
                Ok(w) => Arc::new(w),
                Err(e) => {
                    error!(error = %e, "Failed to build cache worker");
                    continue;
                }
            };

            let mut set = JoinSet::new();
            for ev in batch {
                let cache_worker = Arc::clone(&cache_worker);
                set.spawn(Self::handle_event(cache_worker, ev.event));
            }

            while let Some(res) = set.join_next().await {
                if let Err(e) = res {
                    error!(error = %e, "Failed to handle event");
                }
            }

            // CONCURRENT_EVENTS_GUAGE.add(-batch.len());
        }
    }

    async fn handle_event(cache_worker: Arc<CacheWorker>, raw: Box<RawValue>) -> Result<()> {
        let payload: Dispatch = serde_json::from_str(raw.get())?;

        trace!(?payload, "Received event");

        let event_name = payload.data.to_string();
        EVENT_COUNTER
            .with_label_values(&[event_name.as_str()])
            .inc();

        EVENT_SIZE_HISTOGRAM
            .with_label_values(&[event_name.as_str()])
            .observe(std::mem::size_of_val(&payload.data) as f64);

        let now = Instant::now();

        let mut cachable = true;
        match payload.data {
            Event::ChannelCreate(c) => if let Some(guild_id) = c.guild_id {
                cache_worker.store_channels(vec![c], guild_id).await?
            },
            Event::ChannelUpdate(c) => if let Some(guild_id) = c.guild_id {
                cache_worker.store_channels(vec![c], guild_id).await?
            },
            Event::ChannelDelete(c) => cache_worker.delete_channel(c.id).await?,
            Event::ThreadCreate(t) => if let Some(guild_id) = t.guild_id {
                cache_worker.store_channels(vec![t], guild_id).await?
            },
            Event::ThreadUpdate(t) => {
                if t.thread_metadata
                    .as_ref()
                    .map(|m| m.archived)
                    .unwrap_or(false)
                {
                    cache_worker.delete_channel(t.id).await?
                } else {
                    if let Some(guild_id) = t.guild_id {
                        cache_worker.store_channels(vec![t], guild_id).await?;
                    }
                }
            }
            Event::ThreadDelete(t) => cache_worker.delete_channel(t.id).await?,
            Event::GuildCreate(mut g) => cache_worker.store_guilds(vec![g]).await?,
            Event::GuildUpdate(mut g) => cache_worker.store_guilds(vec![g]).await?,
            Event::GuildDelete(g) => cache_worker.delete_guild(g.id).await?,
            // When removing members, also remove the user, as it's too expensive to check if the user is in another guild.
            // It is cheaper to just fetch the user again later.
            Event::GuildBanAdd(ev) => {
                Self::remove_member_and_user(&cache_worker, ev.user.id, ev.guild_id).await?
            }
            Event::GuildMemberRemove(ev) => {
                Self::remove_member_and_user(&cache_worker, ev.user.id, ev.guild_id).await?
            }
            Event::GuildMemberUpdate(ev) => {
                cache_worker
                    .store_members(
                        vec![Member {
                            user: Some(ev.user),
                            nick: ev.nick,
                            roles: ev.roles,
                            joined_at: ev.joined_at,
                            premium_since: ev.premium_since,
                            deaf: false,
                            mute: false,
                        }],
                        ev.guild_id,
                    )
                    .await?
            }
            Event::GuildMembersChunk(ev) => {
                cache_worker.store_members(ev.members, ev.guild_id).await?
            }
            Event::GuildRoleCreate(ev) => {
                cache_worker.store_roles(vec![ev.role], ev.guild_id).await?
            }
            Event::GuildRoleUpdate(ev) => {
                cache_worker.store_roles(vec![ev.role], ev.guild_id).await?
            }
            Event::GuildRoleDelete(ev) => cache_worker.delete_role(ev.role_id).await?,
            Event::UserUpdate(ev) => cache_worker.store_users(vec![ev]).await?,
            Event::GuildEmojisUpdate(ev) => {
                cache_worker.store_emojis(ev.emojis, ev.guild_id).await?
            }
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

    async fn remove_member_and_user(
        cache_worker: &CacheWorker,
        user_id: Snowflake,
        guild_id: Snowflake,
    ) -> Result<()> {
        cache_worker.delete_member(user_id, guild_id).await?;
        cache_worker.delete_user(user_id).await?;

        Ok(())
    }
}
