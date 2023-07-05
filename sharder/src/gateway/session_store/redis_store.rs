use super::{SessionData, SessionStore};
use crate::{GatewayError, Result};
use async_trait::async_trait;
use deadpool_redis::{
    redis::{AsyncCommands, Pipeline, Value},
    Pool,
};
use std::collections::HashMap;
use std::fmt::Display;
use std::str;
use std::sync::Arc;

pub struct RedisSessionStore {
    client: Arc<Pool>,
    key_prefix: Box<str>,
    expiry_seconds: usize,
}

impl RedisSessionStore {
    pub fn new(client: Arc<Pool>, key_prefix: impl Into<Box<str>>, expiry_seconds: usize) -> Self {
        Self {
            client,
            key_prefix: key_prefix.into(),
            expiry_seconds,
        }
    }

    fn build_key(&self, suffix: impl Display) -> String {
        format!("{}:{}", self.key_prefix, suffix)
    }

    fn build_keys(&self, shard_id: u64) -> Vec<String> {
        vec![
            self.build_key(format!("{shard_id}:sid")),
            self.build_key(format!("{shard_id}:seq")),
            self.build_key(format!("{shard_id}:url")),
        ]
    }

    fn build_data_vec(&self, shard_id: u64, info: SessionData) -> Vec<(String, String)> {
        let mut data = vec![
            (self.build_key(format!("{shard_id}:sid")), info.session_id),
            (
                self.build_key(format!("{shard_id}:seq")),
                info.seq.to_string(),
            ),
        ];

        if let Some(resume_url) = info.resume_url {
            data.push((self.build_key(format!("{shard_id}:url")), resume_url))
        }

        data
    }
}

#[async_trait]
impl SessionStore for RedisSessionStore {
    async fn get(&self, shard_id: u64) -> Result<Option<SessionData>> {
        let mut conn = self.client.get().await?;

        let (session_id, seq, resume_url) = match conn.mget(self.build_keys(shard_id)).await? {
            Value::Bulk(values) => {
                if values.len() != 3 {
                    return GatewayError::WrongResultLength {
                        expected: 3,
                        actual: values.len(),
                    }
                    .into();
                }

                let session_id = match values[0] {
                    Value::Data(ref data) => str::from_utf8(data)?.to_string(),
                    Value::Nil => return Ok(None), // session_id and seq are required to resume
                    _ => return GatewayError::WrongResultType.into(),
                };

                let seq: usize = match values[1] {
                    Value::Data(ref data) => str::from_utf8(data)?.to_string().parse()?,
                    Value::Nil => return Ok(None), // session_id and seq are required to resume
                    _ => return GatewayError::WrongResultType.into(),
                };

                let resume_url = match values[2] {
                    Value::Data(ref data) => Some(str::from_utf8(data)?.to_string()),
                    Value::Nil => None, // resume_url is not required to resume
                    _ => return GatewayError::WrongResultType.into(),
                };

                (session_id, seq, resume_url)
            }
            _ => return GatewayError::WrongResultType.into(),
        };

        Ok(Some(SessionData {
            seq,
            session_id,
            resume_url,
        }))
    }

    async fn get_bulk(&self, shard_ids: &[u64]) -> Result<HashMap<u64, SessionData>> {
        let keys: Vec<_> = shard_ids
            .iter()
            .flat_map(|shard_id| self.build_keys(*shard_id))
            .collect();
        let mut data = HashMap::new();

        let mut conn = self.client.get().await?;

        match conn.mget(keys.as_slice()).await? {
            Value::Bulk(values) => {
                if values.len() != keys.len() {
                    return GatewayError::WrongResultLength {
                        expected: keys.len(),
                        actual: values.len(),
                    }
                    .into();
                }

                for (i, shard_id) in shard_ids.iter().enumerate() {
                    let session_id = match values[i * 3] {
                        Value::Data(ref data) => str::from_utf8(data)?.to_string(),
                        Value::Nil => continue, // session_id and seq are required to resume
                        _ => return GatewayError::WrongResultType.into(),
                    };

                    let seq: usize = match values[i * 3 + 1] {
                        Value::Data(ref data) => str::from_utf8(data)?.to_string().parse()?,
                        Value::Nil => continue, // session_id and seq are required to resume
                        _ => return GatewayError::WrongResultType.into(),
                    };

                    let resume_url = match values[i * 3 + 2] {
                        Value::Data(ref data) => Some(str::from_utf8(data)?.to_string()),
                        Value::Nil => None, // resume_url is not required to resume
                        _ => return GatewayError::WrongResultType.into(),
                    };

                    data.insert(
                        *shard_id,
                        SessionData {
                            seq,
                            session_id,
                            resume_url,
                        },
                    );
                }
            }
            _ => return GatewayError::WrongResultType.into(),
        };

        Ok(data)
    }

    async fn set(&self, shard_id: u64, info: SessionData) -> Result<()> {
        let data = self.build_data_vec(shard_id, info);

        let mut pipeline = Pipeline::new();
        pipeline.atomic().set_multiple(&data[..]);

        for (key, _) in data {
            pipeline.expire(key, self.expiry_seconds);
        }

        let mut conn = self.client.get().await?;
        pipeline.query_async(&mut conn).await?;

        Ok(())
    }

    async fn set_bulk(&self, data: HashMap<u64, SessionData>) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }

        let mapped: Vec<(String, String)> = data
            .into_iter()
            .flat_map(|(shard_id, info)| self.build_data_vec(shard_id, info))
            .collect();

        let mut pipeline = Pipeline::new();
        pipeline.atomic().set_multiple(&mapped[..]);

        for (key, _) in mapped {
            pipeline.expire(key, self.expiry_seconds);
        }

        let mut conn = self.client.get().await?;
        pipeline.query_async(&mut conn).await?;

        Ok(())
    }

    async fn invalidate(&self, shard_id: u64) -> Result<()> {
        let mut conn = self.client.get().await?;
        conn.del(self.build_keys(shard_id)).await?;

        Ok(())
    }

    async fn invalidate_bulk(&self, shard_ids: &[u64]) -> Result<()> {
        let keys: Vec<_> = shard_ids
            .iter()
            .flat_map(|shard_id| self.build_keys(*shard_id))
            .collect();

        let mut conn = self.client.get().await?;
        conn.del(keys.as_slice()).await?;

        Ok(())
    }
}
