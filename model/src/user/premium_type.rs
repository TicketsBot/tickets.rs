use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy)]
#[repr(u8)]
pub enum PremiumType {
    None = 0,
    NitroClassic = 1,
    Nitro = 2,
}
