use async_trait::async_trait;

use crate::gateway::Shard;

use std::collections::HashMap;

use tokio::task::JoinHandle;
use tokio::time::delay_for;
use std::time::Duration;

#[async_trait]
pub trait ShardManager {
    fn connect(shards: HashMap<u16, Shard>) -> Vec<JoinHandle<()>> {
        let mut handles = Vec::with_capacity(shards.len());
        for (i, mut shard) in shards {
            handles.push(tokio::spawn(async move {
                loop {
                    println!("Starting shard {}", i);

                    match shard.connect().await {
                        Ok(()) => println!("Shard {} exited with Ok", i),
                        Err(e) => eprintln!("Shard {} exited with err: {:?}", i, e)
                    }

                    delay_for(Duration::from_millis(500)).await;
                }
            }));
        }

        handles
    }

    fn get_join_handles(&mut self) -> &mut Vec<JoinHandle<()>>;

    async fn start_error_loop(&mut self);
}