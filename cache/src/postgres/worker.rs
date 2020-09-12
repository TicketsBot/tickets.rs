use tokio::sync::{Mutex, mpsc};
use std::sync::Arc;
use crate::postgres::payload::CachePayload;
use model::Snowflake;
use crate::CacheError;
use model::guild::{VoiceState, Guild, Member, Role, Emoji};
use model::channel::Channel;
use model::user::User;
use std::cmp::Ordering::Equal;
use tokio_postgres::Client;

pub struct Worker {
    client: Client,
    rx: PayloadReceiver,
}

type PayloadReceiver = Arc<Mutex<mpsc::Receiver<CachePayload>>>;

impl Worker {
    pub fn new(client: Client, rx: PayloadReceiver) -> Worker {
        Worker { client, rx }
    }

    pub fn start(self) {
        tokio::spawn(async move {
            loop {
                let mut rx = self.rx.lock().await;
                let recv = rx.recv().await;
                drop(rx);

                let payload = match recv {
                    Some(p) => p,
                    None => { // should never happen
                        eprintln!("Cache worker got None");
                        break;
                    }
                };

                // TODO: Handle result
                match payload {
                    CachePayload::Schema { queries, tx } => {
                        let mut batch = String::new();
                        for query in queries {
                            batch.push_str(&query[..]);
                        }

                        if let Err(e) = self.client.batch_execute(&batch[..]).await {
                            let _ = tx.send(Err(CacheError::DatabaseError(e)));
                            break;
                        }
                    }
                    CachePayload::StoreGuilds { guilds, tx } => { let _ = tx.send(self.store_guilds(guilds).await); }
                    CachePayload::GetGuild { id, tx } => { let _ = tx.send(self.get_guild(id).await); }
                    CachePayload::DeleteGuild { id, tx } => { let _ = tx.send(self.delete_guild(id).await); }
                    CachePayload::StoreChannels { channels, tx } => { let _ = tx.send(self.store_channels(channels).await); }
                    CachePayload::GetChannel { id, tx } => { let _ = tx.send(self.get_channel(id).await); }
                    CachePayload::DeleteChannel { id, tx } => { let _ = tx.send(self.delete_channel(id).await); }
                    CachePayload::StoreUsers { users, tx } => { let _ = tx.send(self.store_users(users).await); }
                    CachePayload::GetUser { id, tx } => { let _ = tx.send(self.get_user(id).await); }
                    CachePayload::DeleteUser { id, tx } => { let _ = tx.send(self.delete_user(id).await); }
                    CachePayload::StoreMembers { members, guild_id, tx } => { let _ = tx.send(self.store_members(members, guild_id).await); }
                    CachePayload::GetMember { user_id, guild_id, tx } => { let _ = tx.send(self.get_member(user_id, guild_id).await); }
                    CachePayload::DeleteMember { user_id, guild_id, tx } => { let _ = tx.send(self.delete_member(user_id, guild_id).await); }
                    CachePayload::StoreRoles { roles, guild_id, tx } => { let _ = tx.send(self.store_roles(roles, guild_id).await); }
                    CachePayload::GetRole { id, tx } => { let _ = tx.send(self.get_role(id).await); }
                    CachePayload::DeleteRole { id, tx } => { let _ = tx.send(self.delete_role(id).await); }
                    CachePayload::StoreEmojis { emojis, guild_id, tx } => { let _ = tx.send(self.store_emojis(emojis, guild_id).await); }
                    CachePayload::GetEmoji { id, tx } => { let _ = tx.send(self.get_emoji(id).await); }
                    CachePayload::DeleteEmoji { id, tx } => { let _ = tx.send(self.delete_emoji(id).await); }
                    CachePayload::StoreVoiceState { voice_states, tx } => { let _ = tx.send(self.store_voice_states(voice_states).await); }
                    CachePayload::GetVoiceState { user_id, guild_id, tx } => { let _ = tx.send(self.get_voice_state(user_id, guild_id).await); }
                    CachePayload::DeleteVoiceState { user_id, guild_id, tx } => { let _ = tx.send(self.delete_voice_state(user_id, guild_id).await); }
                    _ => {}
                };
            }
        });
    }
}

impl Worker {
    async fn store_guilds(&self, mut guilds: Vec<Guild>) -> Result<(), CacheError> {
        if guilds.len() == 0 {
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
                query.push_str(",");
            }

            let encoded = serde_json::to_string(guild).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}::jsonb)"#, guild.id.0, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("guild_id") DO UPDATE SET "data" = excluded.data;"#);

        self.client.simple_query(&query[..]).await.map_err(CacheError::DatabaseError)?;

        // cache objects on guild
        let mut res: Result<(), CacheError> = Ok(());

        // TODO: check opts
        for guild in guilds {
            if let Some(channels) = guild.channels {
                if let Err(e) = self.store_channels(channels).await {
                    res = Err(e);
                }
            }

            if let Some(members) = guild.members {
                let users = members
                    .iter()
                    .map(|m| m.user.clone())
                    .filter(|m| m.is_some())
                    .map(|m| m.unwrap())
                    .collect();

                if let Err(e) = self.store_members(members, guild.id).await {
                    res = Err(e);
                }

                if let Err(e) = self.store_users(users).await {
                    res = Err(e)
                }
            }

            if let Err(e) = self.store_roles(guild.roles, guild.id).await {
                res = Err(e);
            }

            // TODO: Check opts
            /*if let Err(e) = self.store_emojis(guild.emojis, guild.id).await {
                res = Err(e)
            }

            if let Some(voice_states) = guild.voice_states {
                if let Err(e) = self.store_voice_states(voice_states).await {
                    res = Err(e)
                }
            }*/
        }

        res
    }

    async fn get_guild(&self, id: Snowflake) -> Result<Option<Guild>, CacheError> {
        Ok(None)
    }

    async fn delete_guild(&self, id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM guilds WHERE "guild_id" = $1;"#;
        self.client.execute(query, &[&(id.0 as i64)]).await.map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    async fn store_channels(&self, channels: Vec<Channel>) -> Result<(), CacheError> {
        let mut channels = channels.into_iter().filter(|c| c.guild_id.is_some()).collect::<Vec<Channel>>();

        if channels.len() == 0 {
            return Ok(());
        }

        channels.sort_by(|c1, c2| c1.id.cmp(&c2.id));
        channels.dedup();

        let mut query = String::from(r#"INSERT INTO channels("channel_id", "guild_id", "data") VALUES"#);

        let mut first = true;
        for channel in channels { // TODO: Cache DMs?
            if first {
                first = false;
            } else {
                query.push_str(",");
            }

            let encoded = serde_json::to_string(&channel).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}, {}::jsonb)"#, channel.id.0, channel.guild_id.unwrap().0, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("channel_id", "guild_id") DO UPDATE SET "data" = excluded.data;"#);

        self.client.simple_query(&query[..]).await.map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_channel(&self, id: Snowflake) -> Result<Option<Channel>, CacheError> {
        Ok(None)
    }

    async fn delete_channel(&self, id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM channels WHERE "channel_id" = $1;"#;
        self.client.execute(query, &[&(id.0 as i64)]).await.map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    async fn store_users(&self, mut users: Vec<User>) -> Result<(), CacheError> {
        if users.len() == 0 {
            return Ok(());
        }

        users.sort_by(|one, two| one.id.cmp(&two.id));
        users.dedup();

        let mut query = String::from(r#"INSERT INTO users("user_id", "data") VALUES"#);

        let mut first = true;
        for user in users {
            if first {
                first = false;
            } else {
                query.push_str(",");
            }

            let encoded = serde_json::to_string(&user).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}::jsonb)"#, user.id.0, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("user_id") DO UPDATE SET "data" = excluded.data;"#);

        self.client.simple_query(&query[..]).await.map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_user(&self, id: Snowflake) -> Result<Option<User>, CacheError> {
        Ok(None)
    }

    async fn delete_user(&self, id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM users WHERE "user_id" = $1;"#;
        self.client.execute(query, &[&(id.0 as i64)]).await.map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    async fn store_members(&self, members: Vec<Member>, guild_id: Snowflake) -> Result<(), CacheError> {
        let mut members = members.into_iter().filter(|m| m.user.is_some()).collect::<Vec<Member>>();

        if members.len() == 0 {
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

        let mut query = String::from(r#"INSERT INTO members("guild_id", "user_id", "data") VALUES"#);

        let mut first = true;
        for member in members {
            if first {
                first = false;
            } else {
                query.push_str(",");
            }

            let encoded = serde_json::to_string(&member).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}, {}::jsonb)"#, guild_id, member.user.as_ref().unwrap().id, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("guild_id", "user_id") DO UPDATE SET "data" = excluded.data;"#);

        self.client.simple_query(&query[..]).await.map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<Member>, CacheError> {
        Ok(None)
    }

    async fn delete_member(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM members WHERE "guild_id" = $1 AND "user_id" = $2;"#;
        self.client.execute(query, &[&(guild_id.0 as i64), &(user_id.0 as i64)]).await.map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    async fn store_roles(&self, mut roles: Vec<Role>, guild_id: Snowflake) -> Result<(), CacheError> {
        if roles.len() == 0 {
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
                query.push_str(",");
            }

            let encoded = serde_json::to_string(&role).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}, {}::jsonb)"#, role.id.0, guild_id.0, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("role_id", "guild_id") DO UPDATE SET "data" = excluded.data;"#);

        self.client.simple_query(&query[..]).await.map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_role(&self, id: Snowflake) -> Result<Option<Role>, CacheError> {
        Ok(None)
    }

    async fn delete_role(&self, id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM roles WHERE "role_id" = $1;"#;
        self.client.execute(query, &[&(id.0 as i64)]).await.map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    async fn store_emojis(&self, emojis: Vec<Emoji>, guild_id: Snowflake) -> Result<(), CacheError> {
        let mut emojis = emojis.into_iter().filter(|e| e.id.is_some()).collect::<Vec<Emoji>>();

        if emojis.len() == 0 {
            return Ok(());
        }

        emojis.sort_by(|e1, e2| e1.id.cmp(&e2.id));
        emojis.dedup();

        let mut query = String::from(r#"INSERT INTO emojis("emoji_id", "guild_id", "data") VALUES"#);

        let mut first = true;
        for emoji in emojis {
            if first {
                first = false;
            } else {
                query.push_str(",");
            }

            let encoded = serde_json::to_string(&emoji).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}, {}::jsonb)"#, emoji.id.unwrap(), guild_id, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("emoji_id", "guild_id") DO UPDATE SET "data" = excluded.data;"#);

        self.client.simple_query(&query[..]).await.map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_emoji(&self, emoji_id: Snowflake) -> Result<Option<Emoji>, CacheError> {
        Ok(None)
    }

    async fn delete_emoji(&self, id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM emojis WHERE "emoji_id" = $1;"#;
        self.client.execute(query, &[&(id.0 as i64)]).await.map_err(CacheError::DatabaseError)?;
        Ok(())
    }

    async fn store_voice_states(&self, voice_states: Vec<VoiceState>) -> Result<(), CacheError> {
        let mut voice_states = voice_states.into_iter().filter(|vs| vs.guild_id.is_some()).collect::<Vec<VoiceState>>();

        if voice_states.len() == 0 {
            return Ok(());
        }

        // TODO: Sort
        voice_states.dedup();

        let mut query = String::from(r#"INSERT INTO voice_states("guild_id", "user_id", "data") VALUES"#);

        let mut first = true;
        for voice_state in voice_states {
            if first {
                first = false;
            } else {
                query.push_str(",");
            }

            let encoded = serde_json::to_string(&voice_state).map_err(CacheError::JsonError)?;
            query.push_str(&format!(r#"({}, {}, {}::jsonb)"#, voice_state.guild_id.unwrap(), voice_state.user_id, quote_literal(encoded)));
        }

        query.push_str(r#" ON CONFLICT("guild_id", "user_id") DO UPDATE SET "data" = excluded.data;"#);

        self.client.simple_query(&query[..]).await.map_err(CacheError::DatabaseError)?;

        Ok(())
    }

    async fn get_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<Option<VoiceState>, CacheError> {
        Ok(None)
    }

    async fn delete_voice_state(&self, user_id: Snowflake, guild_id: Snowflake) -> Result<(), CacheError> {
        let query = r#"DELETE FROM voice_states WHERE "guild_id" = $1 AND "user_id" = $2;"#;
        self.client.execute(query, &[&(guild_id.0 as i64), &(user_id.0 as i64)]).await.map_err(CacheError::DatabaseError)?;
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