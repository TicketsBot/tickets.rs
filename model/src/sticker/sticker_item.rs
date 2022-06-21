use crate::sticker::FormatType;
use crate::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct StickerItem {
    pub id: Snowflake,
    pub name: Box<str>,
    pub format_type: FormatType,
}
