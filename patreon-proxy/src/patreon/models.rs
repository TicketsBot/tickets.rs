use chrono::{DateTime, Utc};
use serde::Deserialize;

use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct PledgeResponse {
    pub data: Vec<Pledge>,
    pub included: Vec<PatronMetadata>,
    pub links: Links,
}

#[derive(Debug, Deserialize)]
pub struct Pledge {
    id: String,
    attributes: PledgeAttributes,
    relationships: Relationships,
}

#[derive(Debug, Deserialize)]
struct PledgeAttributes {
    amount_cents: i32,
    created_at: DateTime<Utc>,
    declined_since: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct Relationships {
    patron: Patron,
    reward: Option<Reward>,
}

#[derive(Debug, Deserialize)]
struct Patron {
    data: PatronData,
}

#[derive(Debug, Deserialize)]
struct PatronData {
    id: String,
}

#[derive(Debug, Deserialize)]
struct Reward {
    data: Option<RewardData>,
}

#[derive(Debug, Deserialize)]
struct RewardData {
    id: String, // tier ID
}

#[derive(Debug, Deserialize)]
pub struct PatronMetadata {
    id: String,
    attributes: PatronAttributes,
}

#[derive(Debug, Deserialize)]
struct PatronAttributes {
    social_connections: Option<SocialConnections>,
}

#[derive(Debug, Deserialize)]
struct SocialConnections {
    discord: Option<DiscordConnection>,
}

#[derive(Debug, Deserialize)]
struct DiscordConnection {
    user_id: String,
}

#[derive(Debug, Deserialize)]
pub struct Links {
    pub next: Option<String>,
}

impl PledgeResponse {
    // returns user ID -> tier
    pub fn convert(&self) -> HashMap<String, super::Tier> {
        self.data
            .iter()
            .filter(|pledge| {
                pledge.attributes.declined_since.is_none() && pledge.relationships.reward.is_some()
            })
            .map(|pledge| -> Option<(String, super::Tier)> {
                let meta = self.get_meta_by_id(&pledge.relationships.patron.data.id)?;
                let discord_id = &meta
                    .attributes
                    .social_connections
                    .as_ref()?
                    .discord
                    .as_ref()?
                    .user_id;
                let tier = super::Tier::get_by_patreon_id(
                    &pledge.relationships.reward.as_ref()?.data.as_ref()?.id,
                )?;

                Some((discord_id.to_string(), tier))
            })
            .filter(|patron| patron.is_some())
            .map(|patron| patron.unwrap())
            .collect()
    }

    fn get_meta_by_id(&self, id: &str) -> Option<&PatronMetadata> {
        self.included.iter().find(|metadata| metadata.id == id)
    }
}
