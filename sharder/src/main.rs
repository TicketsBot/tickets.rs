mod gateway;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut shard = gateway::Shard::new();
    shard.connect().await
}
    