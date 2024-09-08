use chrono::{DateTime, Months, Utc};
use serde::Serialize;

use super::{models::MemberAttributes, tier::TIERS_PREMIUM_LEGACY, tier::TIERS_WHITELABEL_LEGACY, Tier};

#[derive(Debug, Clone, Serialize)]
pub struct Entitlement {
    pub tier: Tier,
    pub label: String,
    pub patreon_tier_id: usize,
    pub is_legacy: bool,
    pub expires_at: DateTime<Utc>,
}

impl Entitlement {
    pub fn new(
        tier: Tier,
        patreon_tier_id: usize,
        is_legacy: bool,
        expires_at: DateTime<Utc>,
    ) -> Entitlement {
        Entitlement {
            label: tier.sku_label().to_string(),
            tier,
            patreon_tier_id,
            is_legacy,
            expires_at,
        }
    }

    pub fn from_patreon_tier_id(
        patreon_id: usize,
        member_attributes: &MemberAttributes,
    ) -> Option<Entitlement> {
        let tier = Tier::get_by_patreon_id(patreon_id)?;

        let is_legacy = match tier {
            Tier::Premium => TIERS_PREMIUM_LEGACY.contains(&patreon_id),
            Tier::Whitelabel => TIERS_WHITELABEL_LEGACY.contains(&patreon_id),
        };

        let expires_at = calculate_expiry(member_attributes);

        Some(Entitlement::new(tier, patreon_id, is_legacy, expires_at))
    }

    pub fn entitled_skus(
        patreon_id: usize,
        member_attributes: &MemberAttributes,
    ) -> Vec<Entitlement> {
        let mut entitlements = vec![];

        if let Some(root) = Self::from_patreon_tier_id(patreon_id, &member_attributes) {
            for inherited in root.tier.inherited_tiers() {
                let expires_at = calculate_expiry(&member_attributes);

                entitlements.push(Entitlement::new(
                    inherited,
                    patreon_id,
                    root.is_legacy,
                    expires_at,
                ));
            }

            entitlements.push(root);
        }

        entitlements
    }
}

fn calculate_expiry(member_attributes: &MemberAttributes) -> DateTime<Utc> {
    if let Some(next_charge_data) = member_attributes.next_charge_date {
        return next_charge_data;
    }

    let last_charge_date = match member_attributes.last_charge_date {
        Some(date) => date,
        None => return Utc::now(),
    };

    let cadence = member_attributes.pledge_cadence.unwrap_or(1);

    last_charge_date
        .checked_add_months(Months::new(cadence.into()))
        .expect("Failed to add months")
}
