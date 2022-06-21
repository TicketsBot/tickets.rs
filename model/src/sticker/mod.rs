use crate::user::User;
use crate::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Sticker {
    pub id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pack_id: Option<Snowflake>,
    pub name: Box<str>,
    pub description: Option<Box<str>>,
    pub tags: Box<str>,
    pub r#type: StickerType,
    pub format_type: FormatType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_value: Option<usize>,
}

mod sticker_type;
pub use sticker_type::StickerType;

mod format_type;
pub use format_type::FormatType;

mod sticker_item;
pub use sticker_item::StickerItem;

mod sticker_pack;
pub use sticker_pack::StickerPack;
