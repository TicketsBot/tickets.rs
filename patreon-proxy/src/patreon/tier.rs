use crate::patreon::tier::Tier::{Premium, Whitelabel};

#[derive(Debug)]
pub enum Tier {
    Premium,
    Whitelabel,
}

// TODO: Don't store these as constants
const TIERS_PREMIUM: &[&str] = &["4071609"];
const TIERS_WHITELABEL: &[&str] = &["5259899", "7502618"];

impl Tier {
    pub fn get_by_patreon_id(patreon_id: &str) -> Option<Tier> {
        if TIERS_WHITELABEL.contains(&patreon_id) {
            Some(Tier::Whitelabel)
        } else if TIERS_PREMIUM.contains(&patreon_id) {
            Some(Tier::Premium)
        } else {
            None
        }
    }

    pub fn tier_id(&self) -> i32 {
        match self {
            Tier::Premium => 0,
            Tier::Whitelabel => 1,
        }
    }

    pub fn values<'a>() -> &'a [Tier] {
        &[Premium, Whitelabel]
    }
}
