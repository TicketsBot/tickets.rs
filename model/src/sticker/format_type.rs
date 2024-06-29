use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Copy, Clone, Debug, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum FormatType {
    Png = 1,
    Apng = 2,
    Lottie = 3,
    Gif = 4,
}
