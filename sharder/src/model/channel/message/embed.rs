use serde::Deserialize;
use chrono::{DateTime, Utc};

#[derive(Deserialize, Debug)]
pub struct Embed {
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub embed_type: EmbedType,
    pub description: Option<String>,
    pub url: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub color: Option<u16>,
    pub footer: Option<EmbedFooter>,
    pub image: Option<EmbedImage>,
    pub thumbnail: Option<EmbedThumbnail>,
    pub video: Option<EmbedVideo>,
    pub provider: Option<EmbedProvider>,
    pub author: Option<EmbedAuthor>,
    pub fields: Option<Vec<EmbedField>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum EmbedType {
    Rich,
    Image,
    Video,
    Gifv,
    Article,
    Link,
}

#[derive(Deserialize, Debug)]
pub struct EmbedFooter {
    pub text: String,
    pub icon_url: Option<String>,
    pub proxy_icon_url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct EmbedImage {
    url: Option<String>,
    proxy_url: Option<String>,
    height: Option<usize>,
    width: Option<usize>,
}

#[derive(Deserialize, Debug)]
pub struct EmbedThumbnail {
    pub url: Option<String>,
    pub proxy_url: Option<String>,
    pub height: Option<usize>,
    pub width: Option<usize>,
}

#[derive(Deserialize, Debug)]
pub struct EmbedVideo {
    pub url: Option<String>,
    pub height: Option<usize>,
    pub width: Option<usize>,
}

#[derive(Deserialize, Debug)]
pub struct EmbedProvider {
    pub name: Option<String>,
    pub url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct EmbedAuthor {
    pub name: Option<String>,
    pub url: Option<String>,
    pub icon_url: Option<String>,
    pub proxy_icon_url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    pub inline: Option<bool>,
}