use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum StatusType {
    Online,
    Dnd,
    Idle,
    Invisible,
    Offline,
}
