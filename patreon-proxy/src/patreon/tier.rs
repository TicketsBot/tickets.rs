use crate::patreon::tier::Tier::{Premium, Whitelabel};

#[derive(Debug)]
pub enum Tier {
    Premium,
    Whitelabel,
}

// TODO: Don't store these as constants
const TIER_PREMIUM: &str = "4071609";
const TIER_WHITELABEL: &str = "5259899";

impl Tier {
    pub fn get_by_patreon_id(patreon_id: &str) -> Option<Tier> {
        match patreon_id {
            TIER_PREMIUM => Some(Tier::Premium),
            TIER_WHITELABEL => Some(Tier::Whitelabel),
            _ => None,
        }
    }

    pub fn tier_id(&self) -> i32 {
        match self {
            Tier::Premium => 0,
            Tier::Whitelabel => 1,
        }
    }

    pub fn values<'a>() -> &'a [Tier] {
        return &[Premium, Whitelabel]
    }
}