use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandInteractionDataOption {
    pub name: Box<str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Box<RawValue>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ApplicationCommandInteractionDataOption>>,
}
