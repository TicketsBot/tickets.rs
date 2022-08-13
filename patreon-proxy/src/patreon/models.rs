use serde::Deserialize;

use crate::patreon::Tier;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct PledgeResponse {
    pub data: Vec<Member>,
    pub included: Vec<PatronMetadata>,
    pub links: Option<Links>,
}

#[derive(Debug, Deserialize)]
pub struct Member {
    attributes: MemberAttributes,
    relationships: Relationships,
}

#[derive(Debug, Deserialize)]
struct MemberAttributes {
    patron_status: Option<PatronStatus>, // null = never pledged
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PatronStatus {
    ActivePatron,
    FormerPatron,
    DeclinedPatron,
}

#[derive(Debug, Deserialize)]
struct Relationships {
    user: User,
    currently_entitled_tiers: EntitledTiers,
}

#[derive(Debug, Deserialize)]
struct User {
    data: UserData,
}

#[derive(Debug, Deserialize)]
struct UserData {
    id: String,
}

#[derive(Debug, Deserialize)]
struct EntitledTiers {
    data: Vec<EntitledTier>,
}

#[derive(Debug, Deserialize)]
struct EntitledTier {
    id: String,
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
    pub fn convert(&self) -> HashMap<String, Tier> {
        self.data
            .iter()
            .filter(|member| {
                member.attributes.patron_status == Some(PatronStatus::ActivePatron)
                    && !member
                        .relationships
                        .currently_entitled_tiers
                        .data
                        .is_empty() // Make sure the user is subscribed to a tier
            })
            .filter_map(|member| -> Option<(String, super::Tier)> {
                let meta = self.get_meta_by_id(member.relationships.user.data.id.as_str())?;

                // Find all subscribed tiers (is this possible?)
                let tier = member
                    .relationships
                    .currently_entitled_tiers
                    .data
                    .iter()
                    .filter_map(|tier| Tier::get_by_patreon_id(tier.id.as_str()))
                    .max_by_key(|tier| tier.tier_id())?;

                let discord_id = meta
                    .attributes
                    .social_connections
                    .as_ref()
                    .and_then(|sc| sc.discord.as_ref())
                    .map(|d| d.user_id.clone())?;

                Some((discord_id, tier))
            })
            .collect()
    }

    fn get_meta_by_id(&self, id: &str) -> Option<&PatronMetadata> {
        self.included.iter().find(|metadata| metadata.id == id)
    }
}
