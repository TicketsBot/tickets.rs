mod shard_manager;
pub use shard_manager::ShardManager;

#[cfg(not(feature = "whitelabel"))]
mod public_shard_manager;
#[cfg(not(feature = "whitelabel"))]
pub use public_shard_manager::PublicShardManager;

#[cfg(feature = "whitelabel")]
mod whitelabel_shard_manager;
#[cfg(feature = "whitelabel")]
pub use whitelabel_shard_manager::WhitelabelShardManager;

mod options;
pub use options::*;

use crate::gateway::Intents;
fn get_intents() -> u64 {
    Intents::build(vec![
        Intents::Guilds,
        Intents::GuildMembers,
        Intents::GuildMessages,
        Intents::MessageContent,
    ])
}
