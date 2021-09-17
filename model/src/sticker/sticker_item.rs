use serde::{Deserialize, Serialize};
use crate::Snowflake;
use crate::sticker::FormatType;

#[derive(Debug, Deserialize, Serialize)]
pub struct StickerItem {
    pub id: Snowflake,
    pub name: Box<str>,
    pub format_type: FormatType,
}
