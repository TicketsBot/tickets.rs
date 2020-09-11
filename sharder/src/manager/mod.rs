mod shard_manager;
pub use shard_manager::ShardManager;

mod public_shard_manager;
pub use public_shard_manager::PublicShardManager;

mod whitelabel_shard_manager;
pub use whitelabel_shard_manager::WhitelabelShardManager;

mod options;
pub use options::*;

mod fatal_error;
pub use fatal_error::FatalError;

use crate::gateway::Intents;
fn get_intents() -> u64 {
    Intents::build(vec![
        Intents::Guilds,
        Intents::GuildMembers,
        Intents::GuildMessages,
        Intents::GuildMessageReactions,
        Intents::DirectMessages,
        Intents::DirectMessageReaction,
    ])
}