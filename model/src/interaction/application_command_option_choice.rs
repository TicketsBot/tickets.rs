use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandOptionChoice {
    pub name: Box<str>,
    pub value: Box<RawValue>, // string, int or float64
}
