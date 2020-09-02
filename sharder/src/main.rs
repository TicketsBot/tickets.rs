mod gateway;
mod model;

use std::error::Error;
use crate::model::user::{StatusUpdate, ActivityType, StatusType};
use gateway::Intents;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let intents = vec![
        Intents::Guilds,
        Intents::GuildMembers,
        Intents::GuildMessages,
        Intents::GuildMessageReactions,
        Intents::DirectMessages,
        Intents::DirectMessageReaction,
    ];

    let mut shard = gateway::Shard::new(gateway::Identify::new(
        String::from(""),
        Some(250),
        gateway::ShardInfo::new(0, 1),
        Some(StatusUpdate::new(ActivityType::Listening, String::from("sex"), StatusType::Online)),
        Intents::build(intents),
    ));

    shard.connect().await
}
    