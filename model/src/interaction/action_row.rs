use super::{Component, ComponentType};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ActionRow {
    pub r#type: ComponentType,
    pub components: Vec<Component>,
}
