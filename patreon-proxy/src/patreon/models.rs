use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_enum_str::Deserialize_enum_str;

use std::collections::HashMap;

use super::Entitlement;

#[derive(Debug, Deserialize)]
pub struct PledgeResponse {
    pub data: Vec<Member>,
    pub included: Vec<PatronMetadata>,
    pub links: Option<Links>,
}

#[derive(Debug, Deserialize)]
pub struct Member {
    pub attributes: MemberAttributes,
    pub relationships: Relationships,
}

#[derive(Debug, Deserialize)]
pub struct MemberAttributes {
    pub last_charge_date: Option<DateTime<Utc>>,
    pub last_charge_status: Option<ChargeStatus>,
    pub next_charge_date: Option<DateTime<Utc>>,
    pub pledge_cadence: Option<u16>,
    pub patron_status: Option<PatronStatus>, // null = never pledged
}

#[derive(Debug, Deserialize_enum_str, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum ChargeStatus {
    Paid,
    Declined,
    Deleted,
    Pending,
    Refunded,
    Fraud,
    Other,
    #[serde(other)]
    Unknown(String),
}

#[derive(Debug, Deserialize_enum_str, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PatronStatus {
    ActivePatron,
    FormerPatron,
    DeclinedPatron,
    #[serde(other)]
    Unknown(String),
}

#[derive(Debug, Deserialize)]
pub struct Relationships {
    pub user: User,
    pub currently_entitled_tiers: EntitledTiers,
}

#[derive(Debug, Deserialize)]
pub struct User {
    pub data: UserData,
}

#[derive(Debug, Deserialize)]
pub struct UserData {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct EntitledTiers {
    pub data: Vec<EntitledTier>,
}

#[derive(Debug, Deserialize)]
pub struct EntitledTier {
    pub id: TierId,
}

#[derive(Copy, Clone, Debug)]
pub struct TierId(pub usize);

#[derive(Debug, Deserialize)]
pub struct PatronMetadata {
    pub id: String,
    pub attributes: PatronAttributes,
}

#[derive(Debug, Deserialize)]
pub struct PatronAttributes {
    pub social_connections: Option<SocialConnections>,
}

#[derive(Debug, Deserialize)]
pub struct SocialConnections {
    pub discord: Option<DiscordConnection>,
}

#[derive(Debug, Deserialize)]
pub struct DiscordConnection {
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Links {
    pub next: Option<String>,
}

impl PledgeResponse {
    // returns user ID -> tier
    pub fn convert(&self) -> HashMap<String, Vec<Entitlement>> {
        self.data
            .iter()
            .filter(|member| {
                member.attributes.patron_status == Some(PatronStatus::ActivePatron)
                    && (member.attributes.last_charge_status == Some(ChargeStatus::Paid)
                        || member.attributes.last_charge_status == Some(ChargeStatus::Pending))
                    && !member
                        .relationships
                        .currently_entitled_tiers
                        .data
                        .is_empty() // Make sure the user is subscribed to a tier
            })
            .filter_map(|member| -> Option<(String, Vec<super::Entitlement>)> {
                let meta = self.get_meta_by_id(member.relationships.user.data.id.as_str())?;

                // Find all subscribed tiers (is this possible?)
                let entitlements = member
                    .relationships
                    .currently_entitled_tiers
                    .data
                    .iter()
                    .map(|tier| Entitlement::entitled_skus(tier.id.into(), &member.attributes))
                    .flatten()
                    .collect();

                let discord_id = meta
                    .attributes
                    .social_connections
                    .as_ref()
                    .and_then(|sc| sc.discord.as_ref())
                    .map(|d| d.user_id.clone())
                    .flatten()?;

                Some((discord_id, entitlements))
            })
            .filter(|(_, skus)| !skus.is_empty())
            .collect()
    }

    fn get_meta_by_id(&self, id: &str) -> Option<&PatronMetadata> {
        self.included.iter().find(|metadata| metadata.id == id)
    }
}

impl<'de> Deserialize<'de> for TierId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let id = String::deserialize(deserializer)?;
        let id = id.parse().map_err(serde::de::Error::custom)?;
        Ok(TierId(id))
    }
}

impl Into<usize> for TierId {
    fn into(self) -> usize {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_charge_status_known() {
        let cases = vec![
            ("Paid", ChargeStatus::Paid),
            ("Declined", ChargeStatus::Declined),
            ("Deleted", ChargeStatus::Deleted),
            ("Pending", ChargeStatus::Pending),
            ("Refunded", ChargeStatus::Refunded),
            ("Fraud", ChargeStatus::Fraud),
            ("Other", ChargeStatus::Other),
        ];

        for (input, expected) in cases {
            let actual = serde_json::from_str::<ChargeStatus>(&format!("\"{}\"", input)).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_deserialize_charge_status_unknown() {
        let actual = serde_json::from_str::<ChargeStatus>("\"Blah Blah\"").unwrap();
        assert_eq!(actual, ChargeStatus::Unknown("Blah Blah".to_string()));
    }

    #[test]
    fn test_deserialize_patron_status_known() {
        let cases = vec![
            ("active_patron", PatronStatus::ActivePatron),
            ("former_patron", PatronStatus::FormerPatron),
            ("declined_patron", PatronStatus::DeclinedPatron),
        ];

        for (input, expected) in cases {
            let actual = serde_json::from_str::<PatronStatus>(&format!("\"{}\"", input)).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_deserialize_patron_status_unknown() {
        let actual = serde_json::from_str::<PatronStatus>("\"Blah Blah\"").unwrap();
        assert_eq!(actual, PatronStatus::Unknown("Blah Blah".to_string()));
    }

    #[test]
    fn test_deserialize_tier_id() {
        let json = r#"{"id": "123456"}"#;
        let unmarshalled = serde_json::from_str::<EntitledTier>(json).unwrap();
        assert_eq!(unmarshalled.id.0, 123456);
    }
}
