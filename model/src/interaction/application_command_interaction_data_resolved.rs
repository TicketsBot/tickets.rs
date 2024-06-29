use crate::channel::message::{Attachment, Message};
use crate::channel::Channel;
use crate::guild::{Member, Role};
use crate::user::User;
use crate::Snowflake;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct ApplicationCommandInteractionDataResolved {
    #[serde(default = "HashMap::new")]
    pub users: HashMap<Snowflake, User>,
    #[serde(default = "HashMap::new")]
    pub members: HashMap<Snowflake, Member>,
    #[serde(default = "HashMap::new")]
    pub roles: HashMap<Snowflake, Role>,
    #[serde(default = "HashMap::new")]
    pub channels: HashMap<Snowflake, Channel>,
    #[serde(default = "HashMap::new")]
    pub messages: HashMap<Snowflake, Message>,
    #[serde(default = "HashMap::new")]
    pub attachments: HashMap<Snowflake, Attachment>,
}

impl Default for ApplicationCommandInteractionDataResolved {
    fn default() -> Self {
        Self {
            users: HashMap::with_capacity(0),
            members: HashMap::with_capacity(0),
            roles: HashMap::with_capacity(0),
            channels: HashMap::with_capacity(0),
            messages: HashMap::with_capacity(0),
            attachments: HashMap::with_capacity(0),
        }
    }
}
