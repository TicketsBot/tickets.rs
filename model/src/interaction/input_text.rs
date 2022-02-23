use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Deserialize, Serialize)]
pub struct InputText {
    pub custom_id: Box<str>,
    pub style: TextStyleType,
    pub label: Box<str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<Box<str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
}

#[derive(Copy, Clone, Debug, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum TextStyleType {
    Short = 1,
    Paragraph = 2,
}
