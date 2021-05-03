use serde_repr::{Serialize_repr, Deserialize_repr};

#[derive(Serialize_repr, Deserialize_repr, Debug, Clone)]
#[repr(u8)]
pub enum VideoQualityMode {
    Auto = 1,
    Full = 2,
}
