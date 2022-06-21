use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Copy, Clone, Debug, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum StickerType {
    Standard = 1,
    Guild = 2,
}
