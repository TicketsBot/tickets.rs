use crate::postgres::payload::CachePayload;
use crate::{CacheError, Options, Result};
use model::channel::Channel;
use model::guild::{Emoji, Guild, Member, Role, VoiceState};
use model::user::User;
use model::Snowflake;
use std::cmp::Ordering::Equal;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_postgres::Client;
use tracing::{debug, error, info, warn};

pub struct Worker {
    id: usize,
    options: Options,
    client: Client,
    rx: PayloadReceiver,
    kill_rx: Mutex<oneshot::Receiver<()>>,
}

pub(crate) type PayloadReceiver = Arc<Mutex<mpsc::UnboundedReceiver<CachePayload>>>;

impl Worker {
    pub fn new(
        id: usize,
        options: Options,
        client: Client,
        rx: PayloadReceiver,
        kill_rx: oneshot::Receiver<()>,
    ) -> Worker {
        Worker {
            id,
            options,
            client,
            rx,
            kill_rx: Mutex::new(kill_rx),
        }
    }

    #[tracing::instrument(name = "start_cache_worker", skip(self))]
    pub fn start(self) {
        info!(id = self.id, "Starting cache worker listener");

        tokio::spawn(async move {
            loop {
                let kill_rx = &mut *self.kill_rx.lock().await;
                let rx = &mut *self.rx.lock().await;

                tokio::select! {
                    _ = kill_rx => {
                        info!(id = self.id, "Shutting down cache worker");
                        break
                    }
                    recv = rx.recv() => {
                        let payload = match recv {
                            Some(p) => p,
                            None => { // Should never happen
                                warn!(id = self.id, "Cache worker receiver dropped");
                                break;
                            }
                        };

                        if let Err(e) = self.handle_payload(payload).await {
                            error!(id = self.id, error = %e, "Failed to handle cache payload")
                        }
                    }
                }
            }
        });
    }

    #[tracing::instrument(name = "Handle cache payload", skip(self, payload))]
    async fn handle_payload(&self, payload: CachePayload) -> Result<()> {
        debug!(id = self.id, payload = ?payload, "Handling cache payload");
        match payload {
            CachePayload::Schema { queries } => {
                let mut batch = String::new();
                for query in queries {
                    batch.push_str(&query[..]);
                }

                Ok(self.client.batch_execute(&batch[..]).await?)
            }
            CachePayload::StoreGuilds { guilds } => self.store_guilds(guilds).await,
            CachePayload::GetGuild { id, tx } => {
                let _ = tx.send(self.get_guild(id).await);
                Ok(())
            }
            CachePayload::DeleteGuild { id } => self.delete_guild(id).await,
            CachePayload::GetGuildCount { tx } => {
                let _ = tx.send(self.get_guild_count().await);
                Ok(())
            }
            CachePayload::StoreChannels { channels } => self.store_channels(channels).await,
            CachePayload::GetChannel { id, tx } => {
                let _ = tx.send(self.get_channel(id).await);
                Ok(())
            }
            CachePayload::DeleteChannel { id } => self.delete_channel(id).await,
            CachePayload::StoreUsers { users } => self.store_users(users).await,
            CachePayload::GetUser { id, tx } => {
                let _ = tx.send(self.get_user(id).await);
                Ok(())
            }
            CachePayload::DeleteUser { id } => self.delete_user(id).await,
            CachePayload::StoreMembers { members, guild_id } => {
                self.store_members(members, guild_id).await
            }
            CachePayload::GetMember {
                user_id,
                guild_id,
                tx,
            } => {
                let _ = tx.send(self.get_member(user_id, guild_id).await);
                Ok(())
            }
            CachePayload::DeleteMember { user_id, guild_id } => {
                self.delete_member(user_id, guild_id).await
            }
            CachePayload::StoreRoles { roles, guild_id } => self.store_roles(roles, guild_id).await,
            CachePayload::GetRole { id, tx } => {
                let _ = tx.send(self.get_role(id).await);
                Ok(())
            }
            CachePayload::DeleteRole { id } => self.delete_role(id).await,
            CachePayload::StoreEmojis { emojis, guild_id } => {
                self.store_emojis(emojis, guild_id).await
            }
            CachePayload::GetEmoji { id, tx } => {
                let _ = tx.send(self.get_emoji(id).await);
                Ok(())
            }
            CachePayload::DeleteEmoji { id } => self.delete_emoji(id).await,
            CachePayload::StoreVoiceState { voice_states } => {
                self.store_voice_states(voice_states).await
            }
            CachePayload::GetVoiceState {
                user_id,
                guild_id,
                tx,
            } => {
                let _ = tx.send(self.get_voice_state(user_id, guild_id).await);
                Ok(())
            }
            CachePayload::DeleteVoiceState { user_id, guild_id } => {
                self.delete_voice_state(user_id, guild_id).await
            }
        }
    }
}

impl Worker {
    #[tracing::instrument(skip(self, guilds))]
    async fn store_guilds(&self, mut guilds: Vec<Guild>) -> Result<()> {
        if guilds.is_empty() {
            return Ok(());
        }

        guilds.sort_by(|g1, g2| g1.id.cmp(&g2.id));
        guilds.dedup();

        let mut query = String::from(r#"INSERT INTO guilds("guild_id", "data") VALUES"#);

        let mut first = true;
        for guild in guilds.iter() {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(guild).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}::jsonb)"#,
                guild.id.0,
                quote_literal(encoded)
            ));
        }

        query.push_str(r#" ON CONFLICT("guild_id") DO UPDATE SET "data" = excluded.data;"#);

        self.client
            .simple_query(&query[..])
            .await
            .map_err(CacheError::DatabaseError)?;

        // cache objects on guild
        let mut res: Result<()> = Ok(());

        for guild in guilds {
            if self.options.channels {
                if let Some(channels) = guild.channels {
                    if let Err(e) = self.store_channels(channels).await {
                        res = Err(e);
                    }
                }
            }

            if self.options.threads {
                if let Some(threads) = guild.threads {
                    if let Err(e) = self.store_channels(threads).await {
                        res = Err(e);
                    }
                }
            }

            if self.options.roles {
                if let Err(e) = self.store_roles(guild.roles, guild.id).await {
                    res = Err(e);
                }
            }

            if self.options.emojis {
                if let Err(e) = self.store_emojis(guild.emojis, guild.id).await {
                    res = Err(e)
                }
            }

            if self.options.voice_states {
                if let Some(voice_states) = guild.voice_states {
                    if let Err(e) = self.store_voice_states(voice_states).await {
                        res = Err(e)
                    }
                }
            }
        }

        res
    }

    async fn get_guild(&self, id: Snowflake) -> Result<Option<Guild>> {
        /*self.client
            .query(
                r#"SELECT "data" FROM guilds WHERE "guild_id" = $1;"#,
                &[&(id.0 as i64)],
            )
            .await;
        Ok(None)*/

        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    async fn delete_guild(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM guilds WHERE "guild_id" = $1;"#;
        self.client
            .execute(query, &[&(id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn get_guild_count(&self) -> Result<usize> {
        let query = r#"SELECT COUNT(guild_id) FROM guilds;"#;

        let row = self
            .client
            .query_one(query, &[])
            .await
            .map_err(CacheError::DatabaseError)?;

        let count: i64 = row.try_get(0).map_err(CacheError::DatabaseError)?;
        Ok(count as usize)
    }

    #[tracing::instrument(skip(self, channels), fields(channel_count = channels.len()))]
    async fn store_channels(&self, channels: Vec<Channel>) -> Result<()> {
        let mut channels = channels
            .into_iter()
            .filter(|c| c.guild_id.is_some())
            .filter(|c| self.options.threads || !c.channel_type.is_thread())
            .collect::<Vec<Channel>>();

        if channels.is_empty() {
            return Ok(());
        }

        channels.sort_by(|c1, c2| c1.id.cmp(&c2.id));
        channels.dedup();

        let mut query =
            String::from(r#"INSERT INTO channels("channel_id", "guild_id", "data") VALUES"#);

        let mut first = true;
        for channel in channels {
            // TODO: Cache DMs?
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&channel).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}, {}::jsonb)"#,
                channel.id.0,
                channel.guild_id.unwrap().0,
                quote_literal(encoded)
            ));
        }

        query.push_str(r#" ON CONFLICT("channel_id") DO UPDATE SET "data" = excluded.data;"#);

        self.client
            .simple_query(&query[..])
            .await
            .map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_channel(&self, id: Snowflake) -> Result<Option<Channel>> {
        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    async fn delete_channel(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM channels WHERE "channel_id" = $1;"#;
        self.client
            .execute(query, &[&(id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, users), fields(user_count = users.len()))]
    async fn store_users(&self, mut users: Vec<User>) -> Result<()> {
        if users.is_empty() {
            return Ok(());
        }

        users.sort_by(|one, two| one.id.cmp(&two.id));
        users.dedup();

        let mut query = String::from(r#"INSERT INTO users("user_id", "data", "last_seen") VALUES"#);

        let mut first = true;
        for user in users {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&user).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}::jsonb, NOW())"#,
                user.id.0,
                quote_literal(encoded)
            ));
        }

        query.push_str(r#" ON CONFLICT("user_id") DO UPDATE SET "data" = excluded.data, "last_seen" = excluded.last_seen;"#);

        self.client
            .simple_query(&query[..])
            .await
            .map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_user(&self, id: Snowflake) -> Result<Option<User>> {
        /*let row = self
            .client
            .query_one(
                r#"SELECT "data" FROM users WHERE "user_id" = $1;"#,
                &[&(id.0 as i64)],
            )
            .await
            .map_err(CacheError::DatabaseError)?;
        let data: &str = row.get(0);

        Ok(None)*/

        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    async fn delete_user(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM users WHERE "user_id" = $1;"#;
        self.client
            .execute(query, &[&(id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, members), fields(member_count = members.len()))]
    async fn store_members(&self, members: Vec<Member>, guild_id: Snowflake) -> Result<()> {
        let mut members = members
            .into_iter()
            .filter(|m| m.user.is_some())
            .collect::<Vec<Member>>();

        if members.is_empty() {
            return Ok(());
        }

        members.sort_by(|one, two| {
            if let (Some(user_one), Some(user_two)) = (&one.user, &two.user) {
                return user_one.id.cmp(&user_two.id);
            }

            Equal // idk
        });

        members.dedup_by(|one, two| {
            if let (Some(user_one), Some(user_two)) = (&one.user, &two.user) {
                return user_one.id == user_two.id;
            }

            false
        });

        let mut query = String::from(
            r#"INSERT INTO members("guild_id", "user_id", "data", "last_seen") VALUES"#,
        );

        let mut first = true;
        for member in members {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&member).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}, {}::jsonb, NOW())"#,
                guild_id,
                member.user.as_ref().unwrap().id,
                quote_literal(encoded)
            ));
        }

        query.push_str(
            r#" ON CONFLICT("guild_id", "user_id") DO UPDATE SET "data" = excluded.data, "last_seen" = excluded.last_seen;"#,
        );

        self.client
            .simple_query(&query[..])
            .await
            .map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<Member>> {
        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    async fn delete_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM members WHERE "guild_id" = $1 AND "user_id" = $2;"#;
        self.client
            .execute(query, &[&(guild_id.0 as i64), &(user_id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, roles), fields(role_count = roles.len()))]
    async fn store_roles(&self, mut roles: Vec<Role>, guild_id: Snowflake) -> Result<()> {
        if roles.is_empty() {
            return Ok(());
        }

        roles.sort_by(|r1, r2| r1.id.cmp(&r2.id));
        roles.dedup();

        let mut query = String::from(r#"INSERT INTO roles("role_id", "guild_id", "data") VALUES"#);

        let mut first = true;
        for role in roles {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&role).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}, {}::jsonb)"#,
                role.id.0,
                guild_id.0,
                quote_literal(encoded)
            ));
        }

        query.push_str(
            r#" ON CONFLICT("role_id", "guild_id") DO UPDATE SET "data" = excluded.data;"#,
        );

        self.client
            .simple_query(&query[..])
            .await
            .map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>> {
        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    async fn delete_role(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM roles WHERE "role_id" = $1;"#;
        self.client
            .execute(query, &[&(id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, emojis), fields(emoji_count = emojis.len()))]
    async fn store_emojis(&self, emojis: Vec<Emoji>, guild_id: Snowflake) -> Result<()> {
        let mut emojis = emojis
            .into_iter()
            .filter(|e| e.id.is_some())
            .collect::<Vec<Emoji>>();

        if emojis.is_empty() {
            return Ok(());
        }

        emojis.sort_by(|e1, e2| e1.id.cmp(&e2.id));
        emojis.dedup();

        let mut query =
            String::from(r#"INSERT INTO emojis("emoji_id", "guild_id", "data") VALUES"#);

        let mut first = true;
        for emoji in emojis {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&emoji).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}, {}::jsonb)"#,
                emoji.id.unwrap(),
                guild_id,
                quote_literal(encoded)
            ));
        }

        query.push_str(
            r#" ON CONFLICT("emoji_id", "guild_id") DO UPDATE SET "data" = excluded.data;"#,
        );

        self.client
            .simple_query(&query[..])
            .await
            .map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_emoji(&self, emoji_id: Snowflake) -> Result<Option<Emoji>> {
        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    async fn delete_emoji(&self, id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM emojis WHERE "emoji_id" = $1;"#;
        self.client
            .execute(query, &[&(id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, voice_states), fields(voice_state_count = voice_states.len()))]
    async fn store_voice_states(&self, voice_states: Vec<VoiceState>) -> Result<()> {
        let mut voice_states = voice_states
            .into_iter()
            .filter(|vs| vs.guild_id.is_some())
            .collect::<Vec<VoiceState>>();

        if voice_states.is_empty() {
            return Ok(());
        }

        // TODO: Sort
        voice_states.dedup();

        let mut query =
            String::from(r#"INSERT INTO voice_states("guild_id", "user_id", "data") VALUES"#);

        let mut first = true;
        for voice_state in voice_states {
            if first {
                first = false;
            } else {
                query.push(',');
            }

            let encoded = serde_json::to_string(&voice_state).map_err(CacheError::JsonError)?;
            query.push_str(&format!(
                r#"({}, {}, {}::jsonb)"#,
                voice_state.guild_id.unwrap(),
                voice_state.user_id,
                quote_literal(encoded)
            ));
        }

        query.push_str(
            r#" ON CONFLICT("guild_id", "user_id") DO UPDATE SET "data" = excluded.data;"#,
        );

        self.client
            .simple_query(&query[..])
            .await
            .map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_voice_state(
        &self,
        user_id: Snowflake,
        guild_id: Snowflake,
    ) -> Result<Option<VoiceState>> {
        unimplemented!()
    }

    #[tracing::instrument(skip(self))]
    async fn delete_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<()> {
        let query = r#"DELETE FROM voice_states WHERE "guild_id" = $1 AND "user_id" = $2;"#;
        self.client
            .execute(query, &[&(guild_id.0 as i64), &(user_id.0 as i64)])
            .await
            .map_err(CacheError::DatabaseError)?;
        Ok(())
    }
}

fn quote_literal(s: String) -> String {
    let s = s.replace("'", "''");

    if s.contains(r#"\"#) {
        let s = s.replace(r#"\"#, r#"\\"#);
        format!(" E'{}'", s)
    } else {
        format!("'{}'", s)
    }
}
