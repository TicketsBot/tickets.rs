use super::Sticker;
use crate::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct StickerPack {
    pub id: Snowflake,
    pub stickers: Vec<Sticker>,
    pub name: Box<str>,
    pub sku_id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_sticker_id: Option<Snowflake>,
    pub description: Box<str>,
    pub banner_asset_id: Snowflake,
}
