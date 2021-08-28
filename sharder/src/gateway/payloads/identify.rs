use super::Opcode;
use crate::gateway::ShardInfo;
use serde::Serialize;

#[derive(Serialize, Debug)]
pub struct Identify {
    #[serde(rename = "op")]
    opcode: Opcode,

    #[serde(rename = "d")]
    pub data: IdentifyData,
}

impl Identify {
    /// large_threshold must be between 50 and 250 inclusive, if Some
    pub fn new(
        token: String,
        large_threshold: Option<i32>,
        shard_info: ShardInfo,
        presence: Option<model::user::StatusUpdate>,
        intents: u64,
    ) -> Identify {
        if let Some(large_threshold) = large_threshold {
            if !(50..=250).contains(&large_threshold) {
                panic!("large_threshold must be between 50 and 250 inclusive");
            }
        }

        Identify {
            opcode: Opcode::Identify,
            data: IdentifyData {
                token,
                properties: ConnectionProperties::new(),
                compress: Some(cfg!(feature = "compression")),
                large_threshold,
                shard_info,
                presence,
                guild_subscriptions: None,
                intents,
            },
        }
    }
}

#[derive(Serialize, Debug)]
pub struct IdentifyData {
    pub token: String,

    pub properties: ConnectionProperties,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_threshold: Option<i32>,

    #[serde(rename = "shard")]
    pub shard_info: ShardInfo,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence: Option<model::user::StatusUpdate>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_subscriptions: Option<bool>,

    pub intents: u64,
}

#[derive(Serialize, Debug)]
pub struct ConnectionProperties {
    #[serde(rename = "$os")]
    pub os: String,

    #[serde(rename = "browser")]
    pub browser: String,

    #[serde(rename = "device")]
    pub device: String,
}

const LIBRARY_NAME: &str = "tickets.rs";

impl ConnectionProperties {
    pub fn new() -> ConnectionProperties {
        ConnectionProperties {
            os: std::env::consts::OS.to_owned(),
            browser: LIBRARY_NAME.to_owned(),
            device: LIBRARY_NAME.to_owned(),
        }
    }
}

impl Default for ConnectionProperties {
    fn default() -> Self {
        Self::new()
    }
}
