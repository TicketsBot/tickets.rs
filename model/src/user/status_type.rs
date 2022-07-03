use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum StatusType {
    Online,
    Dnd,
    Idle,
    Invisible,
    Offline,
}
