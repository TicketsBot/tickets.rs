use async_trait::async_trait;

use super::ShardManager;

use crate::gateway::{Shard, Identify, ShardInfo};

use std::collections::HashMap;
use model::user::{StatusUpdate, ActivityType, StatusType};
use cache::PostgresCache;
use r2d2_redis::RedisConnectionManager;
use r2d2_redis::r2d2::Pool;
use std::sync::Arc;
use database::Database;
use tokio::sync::Mutex;
use model::Snowflake;
use tokio::task::JoinHandle;

pub struct WhitelabelShardManager {

}

impl WhitelabelShardManager {
    pub async fn connect(
        sharder_count: i32,
        sharder_total: i32,
        database: Arc<Database>,
        cache: Arc<PostgresCache>,
        redis: Arc<Pool<RedisConnectionManager>>
    ) -> WhitelabelShardManager {
        // we should panic if we cant read db
        let bots = database.whitelabel.get_bots_by_sharder(sharder_count, sharder_total).await.unwrap();

        let shards: Arc<Mutex<HashMap<u16, Shard>>> = Arc::new(Mutex::new(HashMap::new()));
        let mut handles = Vec::with_capacity(bots.len());
        for bot in bots {
            let shards = Arc::clone(&shards);

            let database = Arc::clone(&database);
            let cache = Arc::clone(&cache);
            let redis = Arc::clone(&redis);

            handles.push(tokio::spawn(async move {
                // retrieve bot status
                let status = match database.whitelabel_status.get(Snowflake(bot.bot_id as u64)).await {
                    Ok(status) => Some(status),
                    Err(sqlx::Error::RowNotFound) => None,
                    Err(e) => {
                        eprintln!("Error occurred while retrieving status for {}: {:?}", bot.bot_id, e);
                        None
                    },
                }.unwrap_or("for t!help".to_owned());

                let shard_info = ShardInfo::new(0, 1);
                let presence = StatusUpdate::new(ActivityType::Listening, status, StatusType::Online);
                let identify = Identify::new(bot.token, None, shard_info, Some(presence), super::get_intents());

                let shard = Shard::new(identify, 1, cache, redis, true);

                let mut shards = shards.lock().await;
                shards.insert(0, shard);
            }));
        }

        futures::future::join_all(handles).await;

        let sm = WhitelabelShardManager {

        };

        let mu = match Arc::try_unwrap(shards) {
            Ok(m) => m,
            Err(e) => panic!(e),
        };

        <WhitelabelShardManager as ShardManager>::connect(mu.into_inner());

        sm
    }
}

#[async_trait]
impl ShardManager for WhitelabelShardManager {
    fn connect(shards: HashMap<u16, Shard>) -> Vec<JoinHandle<()>> {

        vec![]
    }

    fn is_whitelabel() -> bool {
        false
    }

    fn get_join_handles(&mut self) -> &mut Vec<JoinHandle<()>> {
        panic!("monox")
    }
}
