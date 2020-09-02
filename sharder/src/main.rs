mod gateway;
mod model;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut shard = gateway::Shard::new(gateway::Identify::new(

    ));

    shard.connect().await
}
    