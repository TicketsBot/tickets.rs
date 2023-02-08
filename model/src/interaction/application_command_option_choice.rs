use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationCommandOptionChoice {
    pub name: Box<str>,
    pub value: Value, // string, int or float64
}
